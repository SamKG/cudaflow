use std::ffi::CStr;
use std::fmt;

use crate::driver_internal_sys::{cuGetErrorString, CUresult};

#[derive(Debug, Copy, Clone)]
pub struct CudaError {
    pub errcode: CUresult,
}

impl fmt::Display for CudaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            let mut str_ptr: *const std::ffi::c_char = std::ptr::null();
            let result = cuGetErrorString(self.errcode, &mut str_ptr);

            if result == CUresult::CUDA_SUCCESS && !str_ptr.is_null() {
                let c_str = CStr::from_ptr(str_ptr);
                if let Ok(msg) = c_str.to_str() {
                    return write!(f, "{}", msg);
                }
            }
        }

        // Fallback: print the raw error code
        write!(f, "CUDA Error Code: {:?}", self.errcode)
    }
}

impl std::error::Error for CudaError {}

#[macro_export]
macro_rules! cuda_errcheck {
    ($expr:expr) => {{
        let result = $expr;
        if result == $crate::driver_internal_sys::CUresult::CUDA_SUCCESS {
            Ok(())
        } else {
            Err($crate::driver_internal_helpers::CudaError { errcode: result })
        }
    }};
}
