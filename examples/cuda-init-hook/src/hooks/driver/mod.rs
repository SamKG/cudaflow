use cuda_interposer::install_hooks;

pub mod cuinit;

// This macro call generates the cuGetProcAddress hooks and
// uses the map generated in OUT_DIR to link them to the modules above.
install_hooks!();
