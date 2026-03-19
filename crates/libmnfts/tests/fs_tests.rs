use libmnfts::fs::metadata::filetime_to_system_time;
use std::time::SystemTime;

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

// Integration tests below require test NTFS images.
// They are gated with #[ignore] until basic.img is created.

#[ignore]
#[test]
fn test_list_root_directory() {
    use libmnfts::fs::list_directory;
    use libmnfts::volume::NtfsVolume;
    use std::io::Cursor;

    let image_data = std::fs::read("tests/images/basic.img").unwrap();
    let mut volume = NtfsVolume::open(Cursor::new(image_data)).unwrap();
    let entries = list_directory(&mut volume, "/").unwrap();

    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(
        names.contains(&"hello.txt"),
        "Expected hello.txt in root, got: {:?}",
        names
    );
    assert!(
        names.contains(&"Documents"),
        "Expected Documents in root, got: {:?}",
        names
    );
}

#[ignore]
#[test]
fn test_list_subdirectory() {
    use libmnfts::fs::list_directory;
    use libmnfts::volume::NtfsVolume;
    use std::io::Cursor;

    let image_data = std::fs::read("tests/images/basic.img").unwrap();
    let mut volume = NtfsVolume::open(Cursor::new(image_data)).unwrap();
    let entries = list_directory(&mut volume, "/Documents").unwrap();

    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(
        names.contains(&"readme.txt"),
        "Expected readme.txt in Documents, got: {:?}",
        names
    );
}
