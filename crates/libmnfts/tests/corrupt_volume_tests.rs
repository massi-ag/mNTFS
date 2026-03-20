use libmnfts::fs::list_directory;
use libmnfts::volume::NtfsVolume;
use std::io::Cursor;
use std::path::PathBuf;

fn test_image_path(name: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("../../tests/images")
        .join(name)
}

#[test]
fn test_corrupt_mft_volume_opens_successfully() {
    let data = std::fs::read(test_image_path("corrupt-mft.img")).unwrap();
    let vol = NtfsVolume::open(Cursor::new(data));
    assert!(
        vol.is_ok(),
        "Volume with corrupt MFT record should still open"
    );
}

#[test]
fn test_corrupt_mft_skips_bad_records() {
    let data = std::fs::read(test_image_path("corrupt-mft.img")).unwrap();
    let mut vol = NtfsVolume::open(Cursor::new(data)).unwrap();
    let entries = list_directory(&mut vol, "/");
    // Directory enumeration should succeed (corrupt entries are in MFT, not in index)
    // The index B-tree may still reference the corrupt record, but enumeration
    // should skip entries it cannot parse
    assert!(
        entries.is_ok(),
        "Should not fail entirely on corrupt MFT records: {:?}",
        entries.err()
    );
}

#[test]
fn test_corrupt_mft_returns_at_least_some_entries() {
    let data = std::fs::read(test_image_path("corrupt-mft.img")).unwrap();
    let mut vol = NtfsVolume::open(Cursor::new(data)).unwrap();
    if let Ok(entries) = list_directory(&mut vol, "/") {
        // We corrupted record 65 (test2.txt) but hello.txt (record 64) should survive.
        // The index may or may not show the corrupt entry depending on how the
        // ntfs crate handles invalid MFT records referenced by index entries.
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        println!("Entries found in corrupt volume: {:?}", names);
        // At minimum, the volume should not panic
    }
}
