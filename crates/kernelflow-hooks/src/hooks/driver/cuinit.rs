#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types)]

use crate::cuda_hook;

use cust_raw::driver_internal_sys::CUresult;
use tracing::info;

use std::os::raw::c_uint;
cuda_hook! {
    pub unsafe extern "C" fn cuInit(flags: c_uint) -> CUresult {
        let rc = unsafe{(*__real_cuInit)(flags)};
        info!("Initialized CUDA driver!");
        rc
    }
}
