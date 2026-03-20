use thiserror::Error;

#[derive(Debug, Error)]
pub enum MnftsError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("NTFS error: {0}")]
    Ntfs(#[from] ntfs::NtfsError),

    #[error("Volume is not NTFS")]
    NotNtfs,

    #[error("Volume is dirty, mount read-only or run chkdsk on Windows")]
    DirtyVolume,

    #[error("Volume has Windows hibernation active, cannot mount")]
    Hibernated,

    #[error("Unsupported NTFS version: {0}.{1}")]
    UnsupportedVersion(u8, u8),

    #[error("Volume health check failed: {0}")]
    HealthCheck(String),
}

pub type Result<T> = std::result::Result<T, MnftsError>;
