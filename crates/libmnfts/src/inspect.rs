use crate::error::Result;
use ntfs::Ntfs;
use std::io::{Read, Seek};

/// Raw metadata dump for developers.
#[derive(Debug, Clone)]
pub struct VolumeInspection {
    pub ntfs_version: String,
    pub cluster_size: u32,
    pub sector_size: u16,
    pub total_sectors: u64,
    pub volume_label: String,
    pub volume_serial: u64,
    pub dirty_flag: bool,
}

pub fn inspect_volume<R: Read + Seek>(reader: &mut R) -> Result<VolumeInspection> {
    let ntfs = Ntfs::new(reader)?;

    let (ntfs_version, dirty_flag) = ntfs
        .volume_info(reader)
        .map(|info| {
            let version = format!("{}.{}", info.major_version(), info.minor_version());
            let dirty = info
                .flags()
                .contains(ntfs::structured_values::NtfsVolumeFlags::IS_DIRTY);
            (version, dirty)
        })
        .unwrap_or_else(|_| ("unknown".to_string(), false));

    let volume_label = ntfs
        .volume_name(reader)
        .and_then(|r| r.ok())
        .and_then(|n| n.name().to_string().ok())
        .unwrap_or_default();

    Ok(VolumeInspection {
        ntfs_version,
        cluster_size: ntfs.cluster_size(),
        sector_size: ntfs.sector_size(),
        total_sectors: ntfs.size() / ntfs.sector_size() as u64,
        volume_label,
        volume_serial: ntfs.serial_number(),
        dirty_flag,
    })
}

impl std::fmt::Display for VolumeInspection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "NTFS Version:   {}", self.ntfs_version)?;
        writeln!(f, "Volume Label:   {}", self.volume_label)?;
        writeln!(f, "Serial Number:  0x{:016X}", self.volume_serial)?;
        writeln!(f, "Cluster Size:   {} bytes", self.cluster_size)?;
        writeln!(f, "Sector Size:    {} bytes", self.sector_size)?;
        writeln!(f, "Total Sectors:  {}", self.total_sectors)?;
        writeln!(
            f,
            "Dirty Flag:     {}",
            if self.dirty_flag { "YES" } else { "no" }
        )?;
        Ok(())
    }
}
