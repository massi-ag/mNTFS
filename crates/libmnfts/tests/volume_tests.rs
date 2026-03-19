use libmnfts::volume::NtfsVolume;
use std::io::Cursor;

#[test]
fn test_mount_rejects_non_ntfs_image() {
    // 16KB of zeros is not a valid NTFS volume
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
