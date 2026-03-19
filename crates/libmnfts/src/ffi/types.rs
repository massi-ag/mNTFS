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
    ErrInternal = 99,
}

/// Opaque handle to a mounted NTFS volume.
pub struct MnftsVolume {
    _private: [u8; 0],
}
