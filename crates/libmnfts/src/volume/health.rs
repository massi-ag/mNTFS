use crate::error::Result;
use ntfs::Ntfs;
use ntfs::structured_values::NtfsVolumeFlags;
use std::io::{Read, Seek};

/// Flags that indicate volume health issues.
#[derive(Debug, Clone)]
pub struct HealthReport {
    pub dirty_journal: bool,
    pub hibernated: bool,
    pub ntfs_version: (u8, u8),
    pub issues: Vec<String>,
}

impl HealthReport {
    pub fn is_clean(&self) -> bool {
        !self.dirty_journal && !self.hibernated && self.issues.is_empty()
    }
}

/// Check volume health: dirty journal, hibernation, etc.
pub fn check_volume_health<R: Read + Seek>(ntfs: &Ntfs, reader: &mut R) -> Result<HealthReport> {
    let mut report = HealthReport {
        dirty_journal: false,
        hibernated: false,
        ntfs_version: (0, 0),
        issues: Vec::new(),
    };

    // Check volume flags for dirty/hibernation markers
    if let Ok(volume_info) = ntfs.volume_info(reader) {
        report.ntfs_version = (volume_info.major_version(), volume_info.minor_version());

        let flags = volume_info.flags();
        if flags.contains(NtfsVolumeFlags::IS_DIRTY) {
            report.dirty_journal = true;
            report.issues.push("Volume journal is dirty".to_string());
        }
    }

    // Check for Windows hibernation (hiberfil.sys in root)
    // Per errata E4: only flag hibernation if both hiberfil.sys is non-zero AND journal is dirty
    if let Ok(root) = ntfs.root_directory(reader)
        && let Ok(index) = root.directory_index(reader)
    {
        let mut finder = index.finder();
        if let Some(Ok(entry)) =
            ntfs::indexes::NtfsFileNameIndex::find(&mut finder, ntfs, reader, "hiberfil.sys")
            && let Ok(file) = entry.to_file(ntfs, reader)
            && let Some(Ok(data_item)) = file.data(reader, "")
            && let Ok(data_attr) = data_item.to_attribute()
        {
            let size = data_attr.value_length();
            if size > 0 && report.dirty_journal {
                report.hibernated = true;
                report.issues.push(
                    "Windows hibernation/Fast Startup detected (non-zero hiberfil.sys + dirty journal). \
                     Boot Windows and do a full shutdown before mounting."
                        .to_string(),
                );
            }
        }
    }

    Ok(report)
}
