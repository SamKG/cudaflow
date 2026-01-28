use crate::cupti_sys::root::{CUptiResult, cuptiGetErrorMessage};
use std::ffi::CStr;
use std::fmt;

#[derive(Debug, Copy, Clone)]
pub struct CUptiError {
    pub errcode: CUptiResult,
}
impl fmt::Display for CUptiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            let mut str_ptr: *const std::ffi::c_char = std::ptr::null();
            let result = cuptiGetErrorMessage(self.errcode, &mut str_ptr);

            if result == CUptiResult::CUPTI_SUCCESS && !str_ptr.is_null() {
                let c_str = CStr::from_ptr(str_ptr);
                if let Ok(msg) = c_str.to_str() {
                    return write!(f, "{}", msg);
                }
            }
        }

        write!(f, "CUpti Error Code: {:?}", self.errcode)
    }
}
impl std::error::Error for CUptiError {}

#[macro_export]
macro_rules! cupti_errcheck {
    ($expr:expr) => {
        unsafe {
            let result = $expr;
            if result == $crate::cupti_sys::root::CUptiResult::CUPTI_SUCCESS {
                Ok(())
            } else {
                Err($crate::cupti_helpers::CUptiError { errcode: result })
            }
        }
    };
}
