use libmnfts::fs::metadata::filetime_to_system_time;
use std::path::PathBuf;
use std::time::SystemTime;

/// Resolve test image path from workspace root.
fn test_image_path(name: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("../../tests/images")
        .join(name)
}

#[test]
fn test_filetime_to_system_time_epoch() {
    // 2000-01-01T00:00:00 in Windows FILETIME
    // = 125911584000000000 (100ns ticks since 1601-01-01)
    let ft: u64 = 125_911_584_000_000_000;
    let st = filetime_to_system_time(ft).unwrap();
    let unix_secs = st
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // 2000-01-01 = 946684800 unix timestamp
    assert_eq!(unix_secs, 946_684_800);
}

#[test]
fn test_filetime_before_unix_epoch_returns_none() {
    // A FILETIME before 1970-01-01 should return None
    let ft: u64 = 0;
    assert!(filetime_to_system_time(ft).is_none());
}

// Integration tests against basic.img (16MB NTFS with hello.txt and test2.txt)

#[test]
fn test_list_root_directory() {
    use libmnfts::fs::list_directory;
    use libmnfts::volume::NtfsVolume;
    use std::io::Cursor;

    let image_data = std::fs::read(test_image_path("basic.img")).unwrap();
    let mut volume = NtfsVolume::open(Cursor::new(image_data)).unwrap();
    let entries = list_directory(&mut volume, "/").unwrap();

    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(
        names.contains(&"hello.txt"),
        "Expected hello.txt in root, got: {:?}",
        names
    );
    assert!(
        names.contains(&"test2.txt"),
        "Expected test2.txt in root, got: {:?}",
        names
    );
}

#[test]
fn test_list_root_has_timestamps() {
    use libmnfts::fs::list_directory;
    use libmnfts::volume::NtfsVolume;
    use std::io::Cursor;

    let image_data = std::fs::read(test_image_path("basic.img")).unwrap();
    let mut volume = NtfsVolume::open(Cursor::new(image_data)).unwrap();
    let entries = list_directory(&mut volume, "/").unwrap();

    let hello = entries.iter().find(|e| e.name == "hello.txt").unwrap();
    assert!(hello.modified.is_some(), "Expected modified timestamp");
    assert!(hello.created.is_some(), "Expected created timestamp");
    assert!(!hello.is_directory, "hello.txt should not be a directory");
    assert!(hello.file_size > 0, "hello.txt should have non-zero size");
}

#[test]
fn test_read_file_contents() {
    use libmnfts::fs::read_file;
    use libmnfts::volume::NtfsVolume;
    use std::io::Cursor;

    let image_data = std::fs::read(test_image_path("basic.img")).unwrap();
    let mut volume = NtfsVolume::open(Cursor::new(image_data)).unwrap();
    let contents = read_file(&mut volume, "/hello.txt").unwrap();
    assert_eq!(String::from_utf8_lossy(&contents), "Hello, NTFS!\n");
}

#[test]
fn test_read_second_file() {
    use libmnfts::fs::read_file;
    use libmnfts::volume::NtfsVolume;
    use std::io::Cursor;

    let image_data = std::fs::read(test_image_path("basic.img")).unwrap();
    let mut volume = NtfsVolume::open(Cursor::new(image_data)).unwrap();
    let contents = read_file(&mut volume, "/test2.txt").unwrap();
    assert_eq!(
        String::from_utf8_lossy(&contents),
        "Another test file\n"
    );
}

#[test]
fn test_read_nonexistent_file_on_valid_volume() {
    use libmnfts::fs::read_file;
    use libmnfts::volume::NtfsVolume;
    use std::io::Cursor;

    let image_data = std::fs::read(test_image_path("basic.img")).unwrap();
    let mut volume = NtfsVolume::open(Cursor::new(image_data)).unwrap();
    let result = read_file(&mut volume, "/nonexistent.txt");
    assert!(result.is_err());
}

#[test]
fn test_mount_basic_ntfs_image() {
    use libmnfts::volume::NtfsVolume;
    use std::io::Cursor;

    let image_data = std::fs::read(test_image_path("basic.img")).unwrap();
    let volume = NtfsVolume::open(Cursor::new(image_data)).unwrap();
    assert_eq!(volume.label(), "Basic");
    assert!(volume.is_healthy());
}

#[test]
fn test_mount_rejects_non_ntfs() {
    use libmnfts::volume::NtfsVolume;
    use std::io::Cursor;

    let data = vec![0u8; 16 * 1024];
    let result = NtfsVolume::open(Cursor::new(data));
    assert!(result.is_err());
}
