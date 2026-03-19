pub mod types;

use types::MnftsResult;

/// Returns MnftsResult::Ok. Used to verify FFI linkage works.
#[unsafe(no_mangle)]
pub extern "C" fn mnfts_ping() -> MnftsResult {
    MnftsResult::Ok
}

/// Returns the library version as a static C string.
#[unsafe(no_mangle)]
pub extern "C" fn mnfts_version() -> *const std::ffi::c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const std::ffi::c_char
}
