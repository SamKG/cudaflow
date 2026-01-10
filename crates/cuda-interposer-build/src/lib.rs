use anyhow::{Context, Result};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tree_sitter::StreamingIterator;
use walkdir::WalkDir;

pub struct InterposerBuilder {
    src_dir: PathBuf,
    out_dir: PathBuf,
    manifest_dir: PathBuf,
}

impl InterposerBuilder {
    pub fn new() -> Self {
        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        Self {
            src_dir: manifest_dir.join("src"),
            out_dir: PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set")),
            manifest_dir,
        }
    }

    /// Sets the source directory to scan for manual hooks (default: ./src)
    pub fn with_src(mut self, path: impl Into<PathBuf>) -> Self {
        self.src_dir = path.into();
        self
    }

    /// Run the build process.
    pub fn build(self) -> Result<()> {
        println!("cargo:rerun-if-changed={}", self.src_dir.display());

        let manual_hooks = scan_local_hooks(&self.src_dir)?;
        for hook in manual_hooks.keys() {
            println!("cargo:warning=Detected manual hook: {}", hook);
        }

        generate_hook_map(&self.out_dir, &manual_hooks)?;

        let target_dir = find_target_dir(&self.out_dir);
        let all_protos = scan_bindgen_prototypes(&target_dir, &["driver_internal_sys.rs"])?;

        if all_protos.is_empty() {
            println!(
                "cargo:warning=No CUDA prototypes found. Ensure cuda_interposer_sys is in dependencies."
            );
        }

        let mut driver_passthroughs = Vec::new();
        for proto in all_protos {
            // Skip functions explicitly hooked by the user
            if manual_hooks.contains_key(&proto.name) {
                continue;
            }
            // Skip functions handled internally by the interposer infrastructure
            if proto.name == "cuGetProcAddress" || proto.name == "cuGetProcAddress_v2" {
                continue;
            }

            if proto.name.starts_with("cu") {
                driver_passthroughs.push(proto);
            }
        }

        emit_passthroughs(
            &self.out_dir.join("passthroughs_driver.rs"),
            &driver_passthroughs,
        )?;

        Ok(())
    }
}

#[derive(Clone, Debug)]
struct Prototype {
    name: String,
    aliases: Vec<String>,
    args: Vec<(String, String)>, // (Name, Type)
    ret: String,
}

fn scan_local_hooks(root: &Path) -> Result<HashMap<String, String>> {
    let mut hooks = HashMap::new();
    let mut parser = create_rust_parser();
    let macro_query = tree_sitter::Query::new(
        &tree_sitter_rust::LANGUAGE.into(),
        r#"(macro_invocation
            (identifier) @macro_name (#eq? @macro_name "cuda_hook")
            (token_tree) @tt
           )"#,
    )?;
    let func_query = tree_sitter::Query::new(
        &tree_sitter_rust::LANGUAGE.into(),
        r#"(function_item
            name: (identifier) @func_name
            )"#,
    )?;

    for entry in WalkDir::new(root) {
        let entry = entry?;
        if entry.path().extension().map_or(false, |e| e == "rs") {
            let src = fs::read_to_string(entry.path())?;
            let tree = parser
                .parse(&src, None)
                .context("Failed to parse rust file")?;
            let mut cursor = tree_sitter::QueryCursor::new();
            let mut matches = cursor.matches(&macro_query, tree.root_node(), src.as_bytes());

            while let Some(m) = matches.next() {
                let tt_node = m.captures.iter().find(|c| c.index == 1).unwrap().node;
                let tt_text = &src[tt_node.byte_range()];
                if tt_text.len() < 2 {
                    continue;
                }
                let inner_src = &tt_text[1..tt_text.len() - 1];

                // Double Parse: Macro body
                let mut inner_parser = create_rust_parser();
                if let Some(inner_tree) = inner_parser.parse(inner_src, None) {
                    let mut inner_cursor = tree_sitter::QueryCursor::new();
                    let mut inner_matches = inner_cursor.matches(
                        &func_query,
                        inner_tree.root_node(),
                        inner_src.as_bytes(),
                    );
                    while let Some(im) = inner_matches.next() {
                        let name_node = im.captures[0].node;
                        let name = &inner_src[name_node.byte_range()];
                        hooks.insert(name.to_string(), name.to_string());
                    }
                }
            }
        }
    }
    Ok(hooks)
}

fn scan_bindgen_prototypes(root: &Path, filenames: &[&str]) -> Result<Vec<Prototype>> {
    let mut prototypes = HashMap::new();
    let mut parser = create_rust_parser();

    // Find the bindgen output files in target dir
    let valid_files: Vec<PathBuf> = WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            filenames
                .iter()
                .any(|f| e.file_name().to_string_lossy() == *f)
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    for file in valid_files {
        let src = fs::read_to_string(&file)?;
        let tree = parser.parse(&src, None).context("Parse bindgen file")?;
        let query = tree_sitter::Query::new(
            &tree_sitter_rust::LANGUAGE.into(),
            r#"(function_signature_item
                name: (identifier) @name
                parameters: (parameters) @params
                return_type: (_)? @ret
               )"#,
        )?;

        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), src.as_bytes());

        while let Some(m) = matches.next() {
            let name_node = m.captures.iter().find(|c| c.index == 0).unwrap().node;
            let name = get_text(&src, name_node);

            if !name.starts_with("cu") && !name.starts_with("__cuda") {
                continue;
            }

            let params_node = m.captures.iter().find(|c| c.index == 1).unwrap().node;
            let ret_node = m.captures.iter().find(|c| c.index == 2).map(|c| c.node);

            let args = parse_params(&src, params_node);
            let ret = match ret_node {
                Some(n) => get_text(&src, n).trim().to_string(),
                None => "()".to_string(),
            };

            prototypes.entry(name.to_string()).or_insert(Prototype {
                name: name.to_string(),
                aliases: vec![],
                args,
                ret,
            });
        }
    }

    Ok(prototypes.into_values().collect())
}

fn generate_hook_map(out_dir: &Path, hooks: &HashMap<String, String>) -> Result<()> {
    let mut f = fs::File::create(out_dir.join("hook_map.rs"))?;
    writeln!(f, "|name: &str| {{")?;
    writeln!(f, "    match name {{")?;
    for name in hooks.keys() {
        writeln!(f, "        \"{}\" => {{", name)?;
        writeln!(f, "            unsafe extern \"C\" {{ fn {}(); }}", name)?;
        writeln!(
            f,
            "            Some({} as *const () as *mut std::os::raw::c_void)",
            name
        )?;
        writeln!(f, "        }},")?;
    }
    writeln!(f, "        _ => None,")?;
    writeln!(f, "    }}")?;
    writeln!(f, "}}")?;
    Ok(())
}

fn emit_passthroughs(path: &Path, protos: &[Prototype]) -> Result<()> {
    let mut f = fs::File::create(path)?;
    for p in protos {
        let args_str = p
            .args
            .iter()
            .map(|(n, t)| format!("({}: {})", n, t))
            .collect::<Vec<_>>()
            .join(", ");
        let aliases_str = if p.aliases.is_empty() {
            String::new()
        } else {
            format!(", aliases: {}", p.aliases.join(", "))
        };

        writeln!(
            f,
            "cuda_interposer::generate_proxy! {{ fn {}([{}]) -> {}; name: {}{} }}",
            p.name, args_str, p.ret, p.name, aliases_str
        )?;
    }
    Ok(())
}

fn create_rust_parser() -> tree_sitter::Parser {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    parser
}

fn get_text<'a>(src: &'a str, node: tree_sitter::Node) -> &'a str {
    &src[node.byte_range()]
}

fn parse_params(src: &str, node: tree_sitter::Node) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut cursor = node.walk();

    for child in node.named_children(&mut cursor) {
        if child.kind() == "parameter" {
            let pat = child
                .child_by_field_name("pattern")
                .map(|n| get_text(src, n))
                .unwrap_or("_");
            let ty = child
                .child_by_field_name("type")
                .map(|n| get_text(src, n))
                .unwrap_or("c_void");
            out.push((pat.to_string(), ty.to_string()));
        }
    }
    out
}

fn find_target_dir(out_dir: &Path) -> PathBuf {
    let mut p = out_dir.to_path_buf();
    for _ in 0..3 {
        p.pop();
    }
    p
}
