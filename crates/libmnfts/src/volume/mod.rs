pub mod health;

use crate::error::{MnftsError, Result};
use ntfs::Ntfs;
use std::io::{Read, Seek};

/// An open, validated NTFS volume (read-only).
/// The reader is stored alongside Ntfs, and both are accessed
/// through `with_ntfs` to avoid split-borrow conflicts.
pub struct NtfsVolume<R: Read + Seek> {
    ntfs: Ntfs,
    reader: R,
    label: String,
    healthy: bool,
}

impl<R: Read + Seek> NtfsVolume<R> {
    /// Open and validate an NTFS volume from a reader (file, device, image).
    pub fn open(mut reader: R) -> Result<Self> {
        let mut ntfs = Ntfs::new(&mut reader).map_err(|_| MnftsError::NotNtfs)?;

        // Read upcase table for case-insensitive lookups
        ntfs.read_upcase_table(&mut reader)
            .map_err(|_| MnftsError::NotNtfs)?;

        // Run health checks
        let health = health::check_volume_health(&ntfs, &mut reader);
        let healthy = health.as_ref().map(|h| h.is_clean()).unwrap_or(false);

        // Get volume label
        let label = Self::read_label(&ntfs, &mut reader);

        Ok(Self {
            ntfs,
            reader,
            label,
            healthy,
        })
    }

    /// All filesystem operations go through this method, which gives
    /// access to both ntfs and reader without split-borrow issues.
    pub fn with_ntfs<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&Ntfs, &mut R) -> T,
    {
        f(&self.ntfs, &mut self.reader)
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn is_healthy(&self) -> bool {
        self.healthy
    }

    fn read_label(ntfs: &Ntfs, reader: &mut R) -> String {
        ntfs.volume_name(reader)
            .and_then(|r| r.ok())
            .and_then(|name| name.name().to_string().ok())
            .unwrap_or_else(|| "NTFS".to_string())
    }
}
