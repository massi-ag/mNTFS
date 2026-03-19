use crate::error::{MnftsError, Result};
use crate::volume::NtfsVolume;
use ntfs::indexes::NtfsFileNameIndex;
use ntfs::NtfsReadSeek;
use std::io::{Read, Seek, SeekFrom};

/// Read the entire contents of a file by path.
pub fn read_file<R: Read + Seek>(
    volume: &mut NtfsVolume<R>,
    path: &str,
) -> Result<Vec<u8>> {
    volume.with_ntfs(|ntfs, reader| {
        let components: Vec<&str> = path
            .trim_start_matches('/')
            .split('/')
            .filter(|c| !c.is_empty())
            .collect();

        if components.is_empty() {
            return Err(MnftsError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Empty path",
            )));
        }

        // Navigate directories
        let mut current_dir = ntfs.root_directory(reader)?;
        for (i, component) in components.iter().enumerate() {
            let index = current_dir.directory_index(reader)?;
            let mut finder = index.finder();
            let entry =
                NtfsFileNameIndex::find(&mut finder, ntfs, reader, component)
                    .ok_or_else(|| {
                        MnftsError::Io(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("Not found: {}", component),
                        ))
                    })??;

            if i == components.len() - 1 {
                // Last component: read the file's data
                let file = entry.to_file(ntfs, reader)?;
                let data_item = file.data(reader, "").ok_or_else(|| {
                    MnftsError::Io(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "No $DATA attribute",
                    ))
                })??;

                let data_attr = data_item.to_attribute()?;
                let value = data_attr.value(reader)?;
                let len = value.len();

                let mut buf = Vec::with_capacity(len as usize);
                let mut attached = value.attach(reader);
                attached.read_to_end(&mut buf)?;
                return Ok(buf);
            } else {
                current_dir = entry.to_file(ntfs, reader)?;
                if !current_dir.is_directory() {
                    return Err(MnftsError::Io(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("{} is not a directory", component),
                    )));
                }
            }
        }

        Err(MnftsError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("File not found: {}", path),
        )))
    })
}

/// Read a portion of a file by MFT reference (for FSKit paged reads).
pub fn read_file_range<R: Read + Seek>(
    volume: &mut NtfsVolume<R>,
    mft_reference: u64,
    offset: u64,
    length: usize,
) -> Result<Vec<u8>> {
    volume.with_ntfs(|ntfs, reader| {
        let file = ntfs.file(reader, mft_reference)?;

        // Check for compressed files (unsupported in v0.1)
        if let Some(Ok(data_item)) = file.data(reader, "") {
            let data_attr = data_item.to_attribute()?;

            if data_attr.flags().contains(ntfs::NtfsAttributeFlags::COMPRESSED) {
                return Err(MnftsError::Io(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "Compressed NTFS files are not supported. File is visible but cannot be read.",
                )));
            }

            let value = data_attr.value(reader)?;
            let file_size = value.len();

            if offset >= file_size {
                return Ok(Vec::new());
            }

            let actual_length =
                std::cmp::min(length as u64, file_size - offset) as usize;
            let mut buf = vec![0u8; actual_length];

            let mut attached = value.attach(reader);
            attached.seek(SeekFrom::Start(offset))?;
            attached.read_exact(&mut buf)?;
            Ok(buf)
        } else {
            Err(MnftsError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No $DATA attribute",
            )))
        }
    })
}
