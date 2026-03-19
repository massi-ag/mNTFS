use crate::error::Result;
use crate::volume::health::check_volume_health;
use ntfs::Ntfs;
use std::io::{Read, Seek};

/// Human-readable health report.
#[derive(Debug, Clone)]
pub struct DoctorReport {
    pub healthy: bool,
    pub messages: Vec<String>,
}

pub fn check_volume<R: Read + Seek>(reader: &mut R) -> Result<DoctorReport> {
    let mut ntfs = Ntfs::new(reader)?;
    ntfs.read_upcase_table(reader).map_err(|_| {
        crate::error::MnftsError::HealthCheck("Failed to read upcase table".to_string())
    })?;
    let health = check_volume_health(&ntfs, reader)?;

    let mut messages = Vec::new();

    if health.is_clean() {
        messages.push("Volume is clean and healthy.".to_string());
    }

    if health.dirty_journal {
        messages.push(
            "WARNING: Volume journal is dirty. Run chkdsk on Windows before writing.".to_string(),
        );
    }

    if health.hibernated {
        messages.push(
            "WARNING: Windows hibernation detected. Do not write. Boot Windows and shut down properly."
                .to_string(),
        );
    }

    messages.push(format!(
        "NTFS version: {}.{}",
        health.ntfs_version.0, health.ntfs_version.1
    ));

    for issue in &health.issues {
        messages.push(format!("Issue: {}", issue));
    }

    Ok(DoctorReport {
        healthy: health.is_clean(),
        messages,
    })
}

impl std::fmt::Display for DoctorReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for msg in &self.messages {
            writeln!(f, "{}", msg)?;
        }
        Ok(())
    }
}
