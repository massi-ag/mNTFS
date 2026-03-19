use crate::error::{MnftsError, Result};
use crate::fs::metadata::{filetime_to_system_time, DirEntry};
use crate::volume::NtfsVolume;
use ntfs::indexes::NtfsFileNameIndex;
use ntfs::structured_values::{NtfsFileAttributeFlags, NtfsFileNamespace};
use std::io::{Read, Seek};

/// List entries in a directory by path.
pub fn list_directory<R: Read + Seek>(
    volume: &mut NtfsVolume<R>,
    path: &str,
) -> Result<Vec<DirEntry>> {
    volume.with_ntfs(|ntfs, reader| {
        // Navigate to the target directory
        let mut current = ntfs.root_directory(reader)?;

        if path != "/" && !path.is_empty() {
            for component in path.trim_start_matches('/').split('/') {
                if component.is_empty() {
                    continue;
                }
                let index = current.directory_index(reader)?;
                let mut finder = index.finder();
                let entry =
                    NtfsFileNameIndex::find(&mut finder, ntfs, reader, component)
                        .ok_or_else(|| {
                            MnftsError::Io(std::io::Error::new(
                                std::io::ErrorKind::NotFound,
                                format!("Path not found: {}", component),
                            ))
                        })??;
                current = entry.to_file(ntfs, reader)?;
                if !current.is_directory() {
                    return Err(MnftsError::Io(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("{} is not a directory", component),
                    )));
                }
            }
        }

        // Enumerate entries
        let index = current.directory_index(reader)?;
        let mut iter = index.entries();
        let mut entries = Vec::new();

        while let Some(result) = iter.next(reader) {
            let entry = match result {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("Skipping corrupt directory entry: {}", e);
                    continue;
                }
            };

            let file_name = match entry.key() {
                Some(Ok(name)) => name,
                Some(Err(e)) => {
                    tracing::warn!("Skipping entry with unreadable name: {}", e);
                    continue;
                }
                None => continue,
            };

            let name = match file_name.name().to_string() {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Skip NTFS system entries and 8.3 short names
            if name.starts_with('$') || name == "." || name == ".." {
                continue;
            }
            if file_name.namespace() == NtfsFileNamespace::Dos {
                continue;
            }

            let is_directory = file_name.is_directory();
            let file_size = if is_directory {
                0
            } else {
                file_name.data_size()
            };

            let file_attributes = file_name.file_attributes();
            let is_sparse = file_attributes.contains(NtfsFileAttributeFlags::SPARSE_FILE);
            let is_compressed = file_attributes.contains(NtfsFileAttributeFlags::COMPRESSED);
            let is_encrypted = file_attributes.contains(NtfsFileAttributeFlags::ENCRYPTED);

            let created =
                filetime_to_system_time(file_name.creation_time().nt_timestamp());
            let modified =
                filetime_to_system_time(file_name.modification_time().nt_timestamp());
            let accessed =
                filetime_to_system_time(file_name.access_time().nt_timestamp());

            entries.push(DirEntry {
                name,
                is_directory,
                file_size,
                created,
                modified,
                accessed,
                mft_reference: entry.file_reference().file_record_number(),
                is_sparse,
                is_compressed,
                is_encrypted,
            });
        }

        Ok(entries)
    })
}
