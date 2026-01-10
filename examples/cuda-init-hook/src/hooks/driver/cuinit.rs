#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types)]
use cuda_interposer::cuda_hook;
use cuda_interposer_sys::driver_internal_sys::CUresult;
use std::os::raw::c_uint;
use tracing::info;

cuda_hook! {
    pub unsafe extern "C" fn cuInit(flags: c_uint) -> CUresult {
        let rc = unsafe{(*__real_cuInit)(flags)};
        info!("Initialized CUDA driver!");
        rc
    }
}
