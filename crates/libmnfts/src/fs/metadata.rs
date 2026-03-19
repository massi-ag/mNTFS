use std::time::SystemTime;

/// A directory entry with metadata.
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub is_directory: bool,
    pub file_size: u64,
    pub created: Option<SystemTime>,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub mft_reference: u64,
    pub is_sparse: bool,
    pub is_compressed: bool,
    pub is_encrypted: bool,
}

/// Convert NTFS FILETIME (100ns ticks since 1601-01-01) to SystemTime.
pub fn filetime_to_system_time(filetime: u64) -> Option<SystemTime> {
    const EPOCH_DIFF_SECS: u64 = 11_644_473_600;
    const HUNDREDS_NANOS_PER_SEC: u64 = 10_000_000;

    let secs_since_1601 = filetime / HUNDREDS_NANOS_PER_SEC;
    if secs_since_1601 < EPOCH_DIFF_SECS {
        return None;
    }
    let unix_secs = secs_since_1601 - EPOCH_DIFF_SECS;
    let nanos = ((filetime % HUNDREDS_NANOS_PER_SEC) * 100) as u32;
    Some(SystemTime::UNIX_EPOCH + std::time::Duration::new(unix_secs, nanos))
}
