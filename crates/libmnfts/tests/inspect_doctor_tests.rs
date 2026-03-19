use std::io::Cursor;
use std::path::PathBuf;

fn test_image_path(name: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("../../tests/images")
        .join(name)
}

#[test]
fn test_inspect_basic_image() {
    let data = std::fs::read(test_image_path("basic.img")).unwrap();
    let result = libmnfts::inspect::inspect_volume(&mut Cursor::new(data)).unwrap();

    assert_eq!(result.ntfs_version, "3.1");
    assert_eq!(result.volume_label, "Basic");
    assert!(!result.dirty_flag);
    assert!(result.cluster_size > 0);
    assert!(result.sector_size > 0);
    assert!(result.total_sectors > 0);

    // Verify Display impl works
    let display = format!("{}", result);
    assert!(display.contains("NTFS Version:"));
    assert!(display.contains("Basic"));
}

#[test]
fn test_inspect_empty_image() {
    let data = std::fs::read(test_image_path("empty.img")).unwrap();
    let result = libmnfts::inspect::inspect_volume(&mut Cursor::new(data)).unwrap();

    assert_eq!(result.ntfs_version, "3.1");
    assert_eq!(result.volume_label, "Empty");
}

#[test]
fn test_doctor_basic_image() {
    let data = std::fs::read(test_image_path("basic.img")).unwrap();
    let report = libmnfts::doctor::check_volume(&mut Cursor::new(data)).unwrap();

    assert!(report.healthy);
    assert!(
        report.messages.iter().any(|m| m.contains("clean")),
        "Expected 'clean' in messages, got: {:?}",
        report.messages
    );

    // Verify Display impl
    let display = format!("{}", report);
    assert!(display.contains("clean and healthy"));
}

#[test]
fn test_doctor_reports_version() {
    let data = std::fs::read(test_image_path("basic.img")).unwrap();
    let report = libmnfts::doctor::check_volume(&mut Cursor::new(data)).unwrap();

    assert!(
        report.messages.iter().any(|m| m.contains("3.1")),
        "Expected NTFS version in messages, got: {:?}",
        report.messages
    );
}

#[test]
fn test_inspect_non_ntfs_fails() {
    let data = vec![0u8; 16 * 1024];
    let result = libmnfts::inspect::inspect_volume(&mut Cursor::new(data));
    assert!(result.is_err());
}

#[test]
fn test_doctor_non_ntfs_fails() {
    let data = vec![0u8; 16 * 1024];
    let result = libmnfts::doctor::check_volume(&mut Cursor::new(data));
    assert!(result.is_err());
}
