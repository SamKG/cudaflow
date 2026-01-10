use anyhow::{Context, Result};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::StreamingIterator;
use walkdir::WalkDir;

/// Files to scan for "reference" prototypes (from cust_raw)
const BINDINGS_RS: &[&str] = &["driver_internal_sys.rs"];

fn main() -> Result<()> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let target_dir = find_target_dir(&out_dir);
    let src_dir = PathBuf::from("src");

    // 1. Cleanup old shared object to force reload
    let so_path = target_dir.join("libreaper_cuda_hooks.so");
    if so_path.exists() {
        let _ = fs::remove_file(&so_path);
    }

    // 2. Scan local source for manual hooks using Tree-sitter (No Regex)
    let manual_hooks = scan_local_hooks(&src_dir.join("hooks"))?;

    // Debug print to verify detection (visible with cargo build -vv)
    for hook in manual_hooks.keys() {
        println!("cargo:warning=Detected manual hook: {}", hook);
    }

    // 3. Generate hook_map.rs (Match statement for cuGetProcAddress)
    generate_hook_map(&out_dir, &manual_hooks)?;

    // 4. Scan bindgen output for all available CUDA functions
    let all_protos = scan_bindgen_prototypes(&target_dir, BINDINGS_RS)?;

    // 5. Filter: Passthroughs = All - Manual
    let mut driver_passthroughs = Vec::new();
    for proto in all_protos {
        // Skip if we implemented it manually
        if manual_hooks.contains_key(&proto.name) {
            continue;
        }

        if proto.name.starts_with("cu") {
            driver_passthroughs.push(proto);
        }
    }

    // 6. Emit Passthroughs
    emit_passthroughs(
        &out_dir.join("passthroughs_driver.rs"),
        &driver_passthroughs,
    )?;

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src");

    Ok(())
}

// ─── Data Structures ─────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct Prototype {
    name: String,
    aliases: Vec<String>,
    args: Vec<(String, String)>, // (Name, Type)
    ret: String,
}

// ─── Scanners ────────────────────────────────────────────────────────────────

fn scan_local_hooks(root: &Path) -> Result<HashMap<String, String>> {
    let mut hooks = HashMap::new();
    let mut parser = create_rust_parser();

    // Query 1: Find the Macro Invocation and capture its Body (token_tree)
    let macro_query = tree_sitter::Query::new(
        &tree_sitter_rust::LANGUAGE.into(),
        r#"(macro_invocation
            (identifier) @macro_name (#eq? @macro_name "cuda_hook")
            (token_tree) @tt
           )"#,
    )?;

    // Query 2: Find the Function Name inside the body (used in the inner parse)
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
                // Capture @tt (index 1 based on query order above)
                let tt_node = m.captures.iter().find(|c| c.index == 1).unwrap().node;
                let tt_text = &src[tt_node.byte_range()];

                // Strip the surrounding braces `{ ... }` or `( ... )`
                // We assume length >= 2 for braces.
                if tt_text.len() < 2 {
                    continue;
                }
                let inner_src = &tt_text[1..tt_text.len() - 1];

                // DOUBLE PARSE: Parse the macro body content as valid Rust code
                let mut inner_parser = create_rust_parser();
                if let Some(inner_tree) = inner_parser.parse(inner_src, None) {
                    let mut inner_cursor = tree_sitter::QueryCursor::new();
                    let mut inner_matches = inner_cursor.matches(
                        &func_query,
                        inner_tree.root_node(),
                        inner_src.as_bytes(),
                    );

                    while let Some(im) = inner_matches.next() {
                        let name_node = im.captures[0].node; // @func_name is index 0
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

        // Find all extern "C" function signatures
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

            // Filter interesting functions
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

// ─── Emitters ────────────────────────────────────────────────────────────────

fn generate_hook_map(out_dir: &Path, hooks: &HashMap<String, String>) -> Result<()> {
    use std::io::Write;
    let mut f = fs::File::create(out_dir.join("hook_map.rs"))?;
    writeln!(f, "match name {{")?;
    for name in hooks.keys() {
        writeln!(f, "    \"{}\" => {{", name)?;
        writeln!(f, "            unsafe extern \"C\" {{ fn {}(); }}", name)?;
        writeln!(f, "            Some({} as *const () as *mut c_void)", name)?;
        writeln!(f, "    }},")?;
    }
    writeln!(f, "    _ => None,")?;
    writeln!(f, "}}")?;
    Ok(())
}

fn emit_passthroughs(path: &Path, protos: &[Prototype]) -> Result<()> {
    use std::io::Write;
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
            "generate_proxy! {{ fn {}([{}]) -> {}; name: {}{} }}",
            p.name, args_str, p.ret, p.name, aliases_str
        )?;
    }
    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

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
    // Heuristic: ascend until we see 'build' sibling or similar, usually 3 levels up from OUT_DIR
    for _ in 0..3 {
        p.pop();
    }
    p
}
