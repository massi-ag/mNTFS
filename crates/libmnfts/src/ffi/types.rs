/// Result code returned by all FFI functions.
#[repr(C)]
pub enum MnftsResult {
    Ok = 0,
    ErrIo = 1,
    ErrNotNtfs = 2,
    ErrDirtyVolume = 3,
    ErrHibernated = 4,
    ErrUnsupportedVersion = 5,
    ErrHealthCheck = 6,
    ErrNullPointer = 7,
    ErrNotFound = 8,
    ErrUnsupported = 9,
    ErrInternal = 99,
}

/// File metadata returned by mnfts_stat. Caller-allocated.
#[repr(C)]
pub struct MnftsFileInfo {
    pub file_size: u64,
    pub is_directory: bool,
    pub is_sparse: bool,
    pub is_compressed: bool,
    pub is_encrypted: bool,
    pub mft_reference: u64,
    pub created_secs: i64,
    pub modified_secs: i64,
    pub accessed_secs: i64,
}

/// Volume inspection result. Caller-allocated.
#[repr(C)]
pub struct MnftsInspectInfo {
    pub cluster_size: u32,
    pub sector_size: u16,
    pub total_sectors: u64,
    pub volume_serial: u64,
    pub dirty_flag: bool,
    /// NTFS major version
    pub version_major: u8,
    /// NTFS minor version
    pub version_minor: u8,
}

/// Doctor report callback. Called once per message line.
pub type MnftsDoctorCallback =
    unsafe extern "C" fn(context: *mut std::ffi::c_void, message_ptr: *const u8, message_len: u32);

/// Callback for directory enumeration.
/// Rust calls this once per entry. Swift copies what it needs inside the callback.
/// name_ptr is UTF-8 bytes, valid only during the callback.
pub type MnftsDirEntryCallback = unsafe extern "C" fn(
    context: *mut std::ffi::c_void,
    name_ptr: *const u8,
    name_len: u32,
    is_directory: bool,
    file_size: u64,
    mft_reference: u64,
    created_secs: i64,
    modified_secs: i64,
    accessed_secs: i64,
);
