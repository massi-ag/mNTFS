use libmnfts::doctor;
use libmnfts::fs::{list_directory, read_file};
use libmnfts::inspect;
use libmnfts::volume::NtfsVolume;
use std::env;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::os::unix::fs::FileExt;

fn main() {
    let device = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: mnfts-validate <device-or-image>");
        eprintln!("  e.g. mnfts-validate /dev/disk4s1");
        eprintln!("  e.g. mnfts-validate tests/images/basic.img");
        std::process::exit(1);
    });

    println!("=== mNFTS Core Library Validation ===");
    println!("Device: {}\n", device);

    // 1. Inspect
    println!("--- Volume Inspection ---");
    let mut reader = open_device(&device).unwrap_or_else(|e| {
        eprintln!("Failed to open {}: {}", device, e);
        eprintln!("Hint: run with sudo for device access");
        std::process::exit(1);
    });
    match inspect::inspect_volume(&mut reader) {
        Ok(info) => print!("{}", info),
        Err(e) => eprintln!("Inspect failed: {}", e),
    }

    // 2. Doctor
    println!("\n--- Health Check ---");
    let mut reader = open_device(&device).unwrap();
    match doctor::check_volume(&mut reader) {
        Ok(report) => print!("{}", report),
        Err(e) => eprintln!("Doctor failed: {}", e),
    }

    // 3. Mount + list root
    println!("\n--- Root Directory ---");
    let reader = open_device(&device).unwrap();
    match NtfsVolume::open(reader) {
        Ok(mut volume) => {
            println!("Volume label: {}", volume.label());
            println!("Healthy: {}\n", volume.is_healthy());

            match list_directory(&mut volume, "/") {
                Ok(entries) => {
                    println!("{:<40} {:>10} {:>5} MFT#", "Name", "Size", "Dir?");
                    println!("{}", "-".repeat(65));
                    for entry in &entries {
                        println!(
                            "{:<40} {:>10} {:>5} {}",
                            entry.name,
                            if entry.is_directory {
                                "-".to_string()
                            } else {
                                format_size(entry.file_size)
                            },
                            if entry.is_directory { "yes" } else { "" },
                            entry.mft_reference,
                        );
                    }
                    println!("\nTotal: {} entries", entries.len());

                    // 4. Try reading a small text file if any exist
                    if let Some(small_file) = entries
                        .iter()
                        .find(|e| !e.is_directory && e.file_size > 0 && e.file_size < 4096)
                    {
                        println!("\n--- Reading small file: {} ---", small_file.name);
                        let path = format!("/{}", small_file.name);
                        match read_file(&mut volume, &path) {
                            Ok(data) => {
                                if data
                                    .iter()
                                    .all(|&b| b.is_ascii() || b == b'\n' || b == b'\r')
                                {
                                    println!("{}", String::from_utf8_lossy(&data));
                                } else {
                                    println!(
                                        "({} bytes, binary content, first 64 hex bytes:)",
                                        data.len()
                                    );
                                    for (i, byte) in data.iter().take(64).enumerate() {
                                        if i > 0 && i % 16 == 0 {
                                            println!();
                                        }
                                        print!("{:02x} ", byte);
                                    }
                                    println!();
                                }
                            }
                            Err(e) => eprintln!("Read failed: {}", e),
                        }
                    }

                    // 5. Try listing subdirectories
                    for entry in &entries {
                        if entry.is_directory && !entry.name.starts_with("System") {
                            println!("\n--- Subdirectory: {}/ ---", entry.name);
                            let path = format!("/{}", entry.name);
                            match list_directory(&mut volume, &path) {
                                Ok(sub_entries) => {
                                    for sub in &sub_entries {
                                        println!(
                                            "  {:<38} {:>10}",
                                            sub.name,
                                            if sub.is_directory {
                                                "<dir>".to_string()
                                            } else {
                                                format_size(sub.file_size)
                                            }
                                        );
                                    }

                                    // Try reading text files in subdirectory
                                    for sub in &sub_entries {
                                        if !sub.is_directory
                                            && sub.file_size > 0
                                            && sub.file_size < 4096
                                        {
                                            let sub_path = format!("/{}/{}", entry.name, sub.name);
                                            if let Ok(data) = read_file(&mut volume, &sub_path)
                                                && data.iter().all(|&b| {
                                                    b.is_ascii() || b == b'\n' || b == b'\r'
                                                })
                                            {
                                                println!(
                                                    "  Content of {}: {}",
                                                    sub.name,
                                                    String::from_utf8_lossy(&data).trim()
                                                );
                                            }
                                        }
                                    }
                                }
                                Err(e) => eprintln!("  List failed: {}", e),
                            }
                        }
                    }
                }
                Err(e) => eprintln!("List directory failed: {}", e),
            }
        }
        Err(e) => eprintln!("Mount failed: {}", e),
    }

    println!("\n=== Validation complete ===");
}

/// Open a device or image, using aligned I/O for raw devices.
fn open_device(path: &str) -> io::Result<Box<dyn ReadSeek>> {
    if path.starts_with("/dev/rdisk") {
        // Raw device: needs sector-aligned I/O
        let file = File::open(path)?;
        Ok(Box::new(AlignedReader::new(file, 512)?))
    } else if path.starts_with("/dev/disk") {
        // Buffered device (may be busy if mounted), try it
        match File::open(path) {
            Ok(file) => Ok(Box::new(BufReader::with_capacity(64 * 1024, file))),
            Err(e) => {
                // If busy, try the raw device instead
                let raw = path.replace("/dev/disk", "/dev/rdisk");
                eprintln!("Note: {} is busy ({}), trying {}", path, e, raw);
                let file = File::open(&raw)?;
                Ok(Box::new(AlignedReader::new(file, 512)?))
            }
        }
    } else {
        // Regular file
        let file = File::open(path)?;
        Ok(Box::new(BufReader::with_capacity(64 * 1024, file)))
    }
}

/// Trait alias for Read + Seek (needed for Box<dyn>).
trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

/// Sector-aligned reader for macOS raw block devices.
/// All reads to the underlying file are aligned to `sector_size` boundaries.
/// Unaligned reads from consumers are served from an internal buffer.
struct AlignedReader {
    file: File,
    sector_size: u64,
    device_size: u64,
    position: u64,
    buf: Vec<u8>,
    buf_offset: u64, // device offset where buf starts
    buf_len: usize,  // valid bytes in buf
}

impl AlignedReader {
    fn new(file: File, sector_size: u64) -> io::Result<Self> {
        // For raw devices, we can't easily know the size. Handle EOF on read.
        Ok(Self {
            file,
            sector_size,
            device_size: u64::MAX, // will be refined on EOF
            position: 0,
            buf: vec![0u8; sector_size as usize * 128], // 64KB buffer
            buf_offset: u64::MAX,                       // invalid, forces first fill
            buf_len: 0,
        })
    }

    fn fill_buf_at(&mut self, aligned_offset: u64) -> io::Result<()> {
        let n = self.file.read_at(&mut self.buf, aligned_offset)?;
        self.buf_offset = aligned_offset;
        self.buf_len = n;
        if n == 0 {
            self.device_size = aligned_offset;
        }
        Ok(())
    }

    fn align_down(&self, offset: u64) -> u64 {
        offset / self.sector_size * self.sector_size
    }
}

impl Read for AlignedReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() || self.position >= self.device_size {
            return Ok(0);
        }

        let aligned_start = self.align_down(self.position);

        // Check if current position is within our buffer
        let in_buffer = self.buf_offset != u64::MAX
            && self.position >= self.buf_offset
            && self.position < self.buf_offset + self.buf_len as u64;

        if !in_buffer {
            self.fill_buf_at(aligned_start)?;
            if self.buf_len == 0 {
                self.device_size = aligned_start;
                return Ok(0);
            }
        }

        let buf_pos = (self.position - self.buf_offset) as usize;
        let available = self.buf_len - buf_pos;
        let to_copy = buf.len().min(available);

        if to_copy == 0 {
            return Ok(0);
        }

        buf[..to_copy].copy_from_slice(&self.buf[buf_pos..buf_pos + to_copy]);
        self.position += to_copy as u64;
        Ok(to_copy)
    }
}

impl Seek for AlignedReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::Current(offset) => {
                if offset >= 0 {
                    self.position + offset as u64
                } else {
                    self.position.saturating_sub((-offset) as u64)
                }
            }
            SeekFrom::End(_) => {
                // For block devices, we can't easily know the size
                // Try a large seek and see what happens
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "SeekFrom::End not supported on raw devices",
                ));
            }
        };
        self.position = new_pos;
        Ok(new_pos)
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
