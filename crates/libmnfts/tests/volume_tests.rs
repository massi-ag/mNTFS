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
fn test_mount_rejects_non_ntfs_image() {
    let data = vec![0u8; 16 * 1024];
    let result = NtfsVolume::open(Cursor::new(data));
    assert!(result.is_err());
}

#[test]
fn test_mount_rejects_random_data() {
    let data: Vec<u8> = (0..16384).map(|i| (i % 256) as u8).collect();
    let result = NtfsVolume::open(Cursor::new(data));
    assert!(result.is_err());
}

#[test]
fn test_mount_real_ntfs_image() {
    let image_data = std::fs::read(test_image_path("basic.img")).unwrap();
    let volume = NtfsVolume::open(Cursor::new(image_data)).unwrap();
    assert_eq!(volume.label(), "Basic");
    assert!(volume.is_healthy());
}

#[test]
fn test_empty_ntfs_image() {
    let image_data = std::fs::read(test_image_path("empty.img"));
    if let Ok(data) = image_data {
        let volume = NtfsVolume::open(Cursor::new(data)).unwrap();
        assert!(volume.is_healthy());
    }
    // Skip if empty.img doesn't exist yet
}
