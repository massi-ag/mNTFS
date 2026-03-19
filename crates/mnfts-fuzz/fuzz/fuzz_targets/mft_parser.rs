#![no_main]
use libfuzzer_sys::fuzz_target;
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    // Try to open arbitrary bytes as an NTFS volume.
    // Should never panic, only return errors gracefully.
    let _ = libmnfts::volume::NtfsVolume::open(Cursor::new(data.to_vec()));
});
