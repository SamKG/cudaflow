#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types)]

use crate::cuda_hook;
use crate::reexports::driver::cuCtxGetDevice;
use cust_raw::driver_internal_sys::{CUdevice, CUfunction, CUlaunchConfig, CUresult, CUstream};
use std::os::raw::c_void;

cuda_hook! {
    pub unsafe extern "C" fn cuLaunchKernel(
        f: CUfunction,
        gridDimX: u32, gridDimY: u32, gridDimZ: u32,
        blockDimX: u32, blockDimY: u32, blockDimZ: u32,
        sharedMemBytes: u32,
        hStream: CUstream,
        kernelParams: *mut *mut c_void,
        extra: *mut *mut c_void
    ) -> CUresult {
        let mut dev: CUdevice = 0;
        let mut res = unsafe{cuCtxGetDevice(&mut dev as *mut CUdevice)};
        if res != CUresult::CUDA_SUCCESS {
            tracing::warn!("Failed to get current device ordinal, falling back to 0");
        }

        res = unsafe{(*__real_cuLaunchKernel)(
            f,
            gridDimX, gridDimY, gridDimZ,
            blockDimX, blockDimY, blockDimZ,
            sharedMemBytes,
            hStream,
            kernelParams,
            extra
        )};
        res
    }
}

cuda_hook! {
    pub unsafe extern "C" fn cuLaunchKernelEx(
        config: *const CUlaunchConfig,
        f: CUfunction,
        kernelParams: *mut *mut ::std::os::raw::c_void,
        extra: *mut *mut ::std::os::raw::c_void,
    ) -> CUresult {
        let mut dev: CUdevice = 0;
        let mut res = unsafe{cuCtxGetDevice(&mut dev as *mut CUdevice)};
        if res != CUresult::CUDA_SUCCESS {
            tracing::warn!("Failed to get current device ordinal, falling back to 0");
        }

        // FIX: Dereference the lazy static directly
        res = unsafe{(*__real_cuLaunchKernelEx)(
            config,
            f,
            kernelParams,
            extra
        )};
        res
    }
}
