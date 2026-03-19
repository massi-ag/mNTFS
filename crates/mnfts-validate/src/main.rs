use libmnfts::doctor;
use libmnfts::fs::{list_directory, read_file};
use libmnfts::inspect;
use libmnfts::volume::NtfsVolume;
use std::env;
use std::fs::File;

fn main() {
    let device = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: mnfts-validate <device-or-image>");
        eprintln!("  e.g. mnfts-validate /dev/rdisk4s1");
        eprintln!("  e.g. mnfts-validate tests/images/basic.img");
        std::process::exit(1);
    });

    println!("=== mNFTS Core Library Validation ===");
    println!("Device: {}\n", device);

    // 1. Inspect
    println!("--- Volume Inspection ---");
    let mut file = File::open(&device).unwrap_or_else(|e| {
        eprintln!("Failed to open {}: {}", device, e);
        eprintln!("Hint: try /dev/rdisk4s1 (raw device) or run with sudo");
        std::process::exit(1);
    });
    match inspect::inspect_volume(&mut file) {
        Ok(info) => print!("{}", info),
        Err(e) => eprintln!("Inspect failed: {}", e),
    }

    // 2. Doctor
    println!("\n--- Health Check ---");
    let mut file = File::open(&device).unwrap();
    match doctor::check_volume(&mut file) {
        Ok(report) => print!("{}", report),
        Err(e) => eprintln!("Doctor failed: {}", e),
    }

    // 3. Mount + list root
    println!("\n--- Root Directory ---");
    let file = File::open(&device).unwrap();
    match NtfsVolume::open(file) {
        Ok(mut volume) => {
            println!("Volume label: {}", volume.label());
            println!("Healthy: {}\n", volume.is_healthy());

            match list_directory(&mut volume, "/") {
                Ok(entries) => {
                    println!(
                        "{:<40} {:>10} {:>5} {}",
                        "Name", "Size", "Dir?", "MFT#"
                    );
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
                                if data.iter().all(|&b| b.is_ascii() || b == b'\n' || b == b'\r')
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
                }
                Err(e) => eprintln!("List directory failed: {}", e),
            }
        }
        Err(e) => eprintln!("Mount failed: {}", e),
    }

    println!("\n=== Validation complete ===");
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
