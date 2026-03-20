use libmnfts::fs::{list_directory, read_file};
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
fn test_dirty_volume_mounts_successfully() {
    let data = std::fs::read(test_image_path("dirty.img")).unwrap();
    let vol = NtfsVolume::open(Cursor::new(data));
    assert!(vol.is_ok(), "Dirty volume should mount successfully");
}

#[test]
fn test_dirty_volume_reports_unhealthy() {
    let data = std::fs::read(test_image_path("dirty.img")).unwrap();
    let vol = NtfsVolume::open(Cursor::new(data)).unwrap();
    assert!(!vol.is_healthy(), "Dirty volume should report unhealthy");
}

#[test]
fn test_dirty_volume_can_list_files() {
    let data = std::fs::read(test_image_path("dirty.img")).unwrap();
    let mut vol = NtfsVolume::open(Cursor::new(data)).unwrap();
    let entries = list_directory(&mut vol, "/");
    assert!(
        entries.is_ok(),
        "Should be able to list files from dirty volume"
    );
    assert!(
        !entries.unwrap().is_empty(),
        "Dirty volume should still have files"
    );
}

#[test]
fn test_dirty_volume_can_read_files() {
    let data = std::fs::read(test_image_path("dirty.img")).unwrap();
    let mut vol = NtfsVolume::open(Cursor::new(data)).unwrap();
    let contents = read_file(&mut vol, "/hello.txt");
    assert!(
        contents.is_ok(),
        "Should be able to read files from dirty volume"
    );
    assert_eq!(
        String::from_utf8_lossy(&contents.unwrap()),
        "Hello, NTFS!\n"
    );
}

#[test]
fn test_dirty_volume_doctor_reports_dirty() {
    let data = std::fs::read(test_image_path("dirty.img")).unwrap();
    let report = libmnfts::doctor::check_volume(&mut Cursor::new(data)).unwrap();
    assert!(
        !report.healthy,
        "Doctor should report unhealthy for dirty volume"
    );
    assert!(
        report.messages.iter().any(|m| m.contains("dirty")),
        "Doctor should mention dirty journal, got: {:?}",
        report.messages
    );
}

#[test]
fn test_dirty_volume_inspect_shows_dirty_flag() {
    let data = std::fs::read(test_image_path("dirty.img")).unwrap();
    let info = libmnfts::inspect::inspect_volume(&mut Cursor::new(data)).unwrap();
    assert!(info.dirty_flag, "Inspect should show dirty flag");
}

#[test]
fn test_clean_volume_is_not_dirty() {
    let data = std::fs::read(test_image_path("basic.img")).unwrap();
    let info = libmnfts::inspect::inspect_volume(&mut Cursor::new(data)).unwrap();
    assert!(!info.dirty_flag, "Clean volume should not have dirty flag");
}
