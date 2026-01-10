// The entry point is much cleaner now.
pub mod ctor;
pub mod hooks;

pub mod reexports {
    pub mod driver {
        #![allow(non_snake_case, clippy::missing_safety_doc)]
        use cuda_interposer_sys::driver_internal_sys::*;
        include!(concat!(env!("OUT_DIR"), "/passthroughs_driver.rs"));
    }
}
