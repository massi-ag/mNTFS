use crate::error::Result;
use std::io::{Read, Seek, SeekFrom};

/// Reads fixed-size blocks from an underlying reader.
/// The reader can be a file, disk image, or FSKit-provided handle.
pub struct BlockReader<R: Read + Seek> {
    reader: std::cell::RefCell<R>,
    block_size: u32,
    total_size: u64,
}

impl<R: Read + Seek> BlockReader<R> {
    pub fn from_reader(mut reader: R, block_size: u32) -> Self {
        let total_size = reader.seek(SeekFrom::End(0)).unwrap_or(0);
        reader.seek(SeekFrom::Start(0)).ok();
        Self {
            reader: std::cell::RefCell::new(reader),
            block_size,
            total_size,
        }
    }

    pub fn block_size(&self) -> u32 {
        self.block_size
    }

    pub fn total_blocks(&self) -> u64 {
        self.total_size / self.block_size as u64
    }

    pub fn read_block(&self, block_num: u64) -> Result<Vec<u8>> {
        let offset = block_num * self.block_size as u64;
        if offset + self.block_size as u64 > self.total_size {
            return Err(crate::error::MnftsError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!(
                    "Block {} out of range (total: {})",
                    block_num,
                    self.total_blocks()
                ),
            )));
        }

        let mut reader = self.reader.borrow_mut();
        reader.seek(SeekFrom::Start(offset))?;
        let mut buf = vec![0u8; self.block_size as usize];
        reader.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn read_bytes(&self, offset: u64, length: usize) -> Result<Vec<u8>> {
        if offset + length as u64 > self.total_size {
            return Err(crate::error::MnftsError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!(
                    "Read at offset {} length {} exceeds volume size {}",
                    offset, length, self.total_size
                ),
            )));
        }

        let mut reader = self.reader.borrow_mut();
        reader.seek(SeekFrom::Start(offset))?;
        let mut buf = vec![0u8; length];
        reader.read_exact(&mut buf)?;
        Ok(buf)
    }
}
