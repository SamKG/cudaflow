pub mod ctor;
pub mod hooks;
pub mod reexports;

use std::{
    env,
    ffi::{CStr, CString},
    os::raw::c_void,
    sync::OnceLock,
};
use tracing::debug;

// ─── Library Loading ─────────────────────────────────────────────────────────

// NEW: Wrapper to make the pointer Sync + Send
struct DlHandle(*mut c_void);
unsafe impl Send for DlHandle {}
unsafe impl Sync for DlHandle {}

// CHANGED: Use the wrapper here
static CUDA_LIB: OnceLock<DlHandle> = OnceLock::new();

/// Returns the raw handle to the real libcuda.so
fn get_libcuda() -> *mut c_void {
    // CHANGED: Unwrap the internal pointer
    let handle_wrapper = CUDA_LIB.get_or_init(|| unsafe {
        let mut paths = vec![
            "/usr/local/cuda/compat/libcuda.so".to_string(),
            "/usr/lib/x86_64-linux-gnu/libcuda.so".to_string(),
            "/usr/lib64/libcuda.so".to_string(),
            "/usr/local/cuda/targets/x86_64-linux/lib/stubs/libcuda.so".to_string(),
        ];
        if let Some(cuda_home) = env::var_os("CUDA_HOME") {
            let path = format!("{}/compat/libcuda.so", cuda_home.to_string_lossy());
            paths.insert(0, path);
        }

        for path in paths.iter() {
            let s = CString::new(path.clone()).unwrap();
            let flags = libc::RTLD_NOW | libc::RTLD_LOCAL | libc::RTLD_NODELETE;
            let handle = libc::dlopen(s.as_ptr(), flags);
            if !handle.is_null() {
                debug!("Loaded real CUDA driver from: {}", path);
                return DlHandle(handle); // Wrap it
            }
        }
        panic!("Failed to find/load libcuda.so. Ensure it is in LD_LIBRARY_PATH.");
    });

    handle_wrapper.0
}

/// dlsym wrapper
pub fn dlsym_next(symbol: &[u8]) -> *mut c_void {
    let handle = get_libcuda();
    unsafe {
        let ptr = libc::dlsym(handle, symbol.as_ptr() as *const _);
        ptr
    }
}

// ─── Hook Macro ──────────────────────────────────────────────────────────────
#[macro_export]
macro_rules! cuda_hook {
    (
        pub unsafe extern "C" fn $fname:ident( $($arg:ident : $arg_ty:ty),* $(,)? )
        -> $ret:ty
        $body:block
    ) => {
        paste::paste! {
            // Lazy pointer to the REAL implementation (for trampoline calls)
            #[allow(non_upper_case_globals)]
            pub static [<__real_ $fname>]: ::once_cell::sync::Lazy<
                unsafe extern "C" fn($($arg_ty),*) -> $ret
            > = ::once_cell::sync::Lazy::new(|| {
                let name = concat!(stringify!($fname), "\0");
                let sym = $crate::dlsym_next(name.as_bytes());
                if sym.is_null() {
                    panic!("Missing symbol: {}", stringify!($fname));
                }
                unsafe { std::mem::transmute(sym) }
            });

            // The Interposer
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $fname( $($arg : $arg_ty),* ) -> $ret {
                 $body
            }
        }
    };
}
