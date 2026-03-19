// FFI module: all public functions are C-ABI exports called from Swift.
// Safety invariants are documented per-function where relevant.
#![allow(clippy::missing_safety_doc)]

pub mod types;

use std::io::{BufReader, Read, Seek};
use std::sync::Mutex;
use types::{MnftsDirEntryCallback, MnftsFileInfo, MnftsResult};

use crate::error::MnftsError;
use crate::fs::{list_directory, read_file_range};
use crate::volume::NtfsVolume;

/// Trait alias for Read + Seek + Send.
trait ReadSeekSend: Read + Seek + Send {}
impl<T: Read + Seek + Send> ReadSeekSend for T {}

/// Opaque FFI handle wrapping the volume in a Mutex for thread safety.
/// FSKit may dispatch callbacks from different threads.
pub struct MnftsVolumeHandle {
    inner: Mutex<NtfsVolume<Box<dyn ReadSeekSend>>>,
}

/// Convert MnftsError to MnftsResult.
fn error_to_result(err: &MnftsError) -> MnftsResult {
    match err {
        MnftsError::Io(e) => match e.kind() {
            std::io::ErrorKind::NotFound => MnftsResult::ErrNotFound,
            std::io::ErrorKind::Unsupported => MnftsResult::ErrUnsupported,
            _ => MnftsResult::ErrIo,
        },
        MnftsError::Ntfs(_) => MnftsResult::ErrIo,
        MnftsError::NotNtfs => MnftsResult::ErrNotNtfs,
        MnftsError::DirtyVolume => MnftsResult::ErrDirtyVolume,
        MnftsError::Hibernated => MnftsResult::ErrHibernated,
        MnftsError::UnsupportedVersion(_, _) => MnftsResult::ErrUnsupportedVersion,
        MnftsError::HealthCheck(_) => MnftsResult::ErrHealthCheck,
    }
}

/// Convert SystemTime to unix seconds, or -1 if unknown.
fn system_time_to_secs(t: Option<std::time::SystemTime>) -> i64 {
    t.and_then(|st| {
        st.duration_since(std::time::SystemTime::UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs() as i64)
    })
    .unwrap_or(-1)
}

// --- Ping / Version (unchanged) ---

/// Returns MnftsResult::Ok. Used to verify FFI linkage works.
#[unsafe(no_mangle)]
pub extern "C" fn mnfts_ping() -> MnftsResult {
    MnftsResult::Ok
}

/// Returns the library version as a static C string.
#[unsafe(no_mangle)]
pub extern "C" fn mnfts_version() -> *const std::ffi::c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const std::ffi::c_char
}

// --- Volume lifecycle ---

/// Open an NTFS volume from a file descriptor.
/// Returns an opaque handle, or null on failure.
/// The caller must eventually call mnfts_close_volume.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mnfts_open_volume(fd: i32) -> *mut MnftsVolumeHandle {
    let result = std::panic::catch_unwind(|| {
        use std::os::unix::io::FromRawFd;
        // Duplicate the fd so Rust owns its own copy
        let new_fd = unsafe { libc::dup(fd) };
        if new_fd < 0 {
            return std::ptr::null_mut();
        }
        let file = unsafe { std::fs::File::from_raw_fd(new_fd) };
        let reader: Box<dyn ReadSeekSend> = Box::new(BufReader::with_capacity(64 * 1024, file));
        match NtfsVolume::open(reader) {
            Ok(volume) => {
                let handle = Box::new(MnftsVolumeHandle {
                    inner: Mutex::new(volume),
                });
                Box::into_raw(handle)
            }
            Err(_) => std::ptr::null_mut(),
        }
    });
    result.unwrap_or(std::ptr::null_mut())
}

/// Close a volume handle and free its resources.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mnfts_close_volume(handle: *mut MnftsVolumeHandle) {
    if !handle.is_null() {
        drop(unsafe { Box::from_raw(handle) });
    }
}

/// Get the volume label. Writes UTF-8 into the provided buffer.
/// Returns the number of bytes written, or 0 on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mnfts_volume_label(
    handle: *const MnftsVolumeHandle,
    buf: *mut u8,
    buf_len: u32,
) -> u32 {
    if handle.is_null() || buf.is_null() {
        return 0;
    }
    let handle = unsafe { &*handle };
    let volume = match handle.inner.lock() {
        Ok(v) => v,
        Err(_) => return 0,
    };
    let label = volume.label();
    let bytes = label.as_bytes();
    let copy_len = bytes.len().min(buf_len as usize);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf, copy_len);
    }
    copy_len as u32
}

/// Check if the volume is healthy.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mnfts_volume_is_healthy(handle: *const MnftsVolumeHandle) -> bool {
    if handle.is_null() {
        return false;
    }
    let handle = unsafe { &*handle };
    let volume = match handle.inner.lock() {
        Ok(v) => v,
        Err(_) => return false,
    };
    volume.is_healthy()
}

// --- Directory enumeration (callback-based, per errata E2) ---

/// Enumerate directory entries at the given path.
/// Calls `callback` once per entry. Rust owns all memory.
/// Swift copies what it needs inside the callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mnfts_readdir(
    handle: *mut MnftsVolumeHandle,
    path_ptr: *const u8,
    path_len: u32,
    callback: MnftsDirEntryCallback,
    context: *mut std::ffi::c_void,
) -> MnftsResult {
    if handle.is_null() || path_ptr.is_null() {
        return MnftsResult::ErrNullPointer;
    }

    let result = std::panic::catch_unwind(|| {
        let handle = unsafe { &*handle };
        let path_bytes = unsafe { std::slice::from_raw_parts(path_ptr, path_len as usize) };
        let path = match std::str::from_utf8(path_bytes) {
            Ok(s) => s,
            Err(_) => return MnftsResult::ErrIo,
        };

        let mut volume = match handle.inner.lock() {
            Ok(v) => v,
            Err(_) => return MnftsResult::ErrInternal,
        };

        match list_directory(&mut volume, path) {
            Ok(entries) => {
                for entry in &entries {
                    let name_bytes = entry.name.as_bytes();
                    unsafe {
                        callback(
                            context,
                            name_bytes.as_ptr(),
                            name_bytes.len() as u32,
                            entry.is_directory,
                            entry.file_size,
                            entry.mft_reference,
                            system_time_to_secs(entry.created),
                            system_time_to_secs(entry.modified),
                            system_time_to_secs(entry.accessed),
                        );
                    }
                }
                MnftsResult::Ok
            }
            Err(e) => error_to_result(&e),
        }
    });

    result.unwrap_or(MnftsResult::ErrInternal)
}

// --- File stat ---

/// Get file metadata by MFT reference. Fills caller-allocated MnftsFileInfo.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mnfts_stat(
    handle: *mut MnftsVolumeHandle,
    path_ptr: *const u8,
    path_len: u32,
    out: *mut MnftsFileInfo,
) -> MnftsResult {
    if handle.is_null() || path_ptr.is_null() || out.is_null() {
        return MnftsResult::ErrNullPointer;
    }

    let result = std::panic::catch_unwind(|| {
        let handle = unsafe { &*handle };
        let path_bytes = unsafe { std::slice::from_raw_parts(path_ptr, path_len as usize) };
        let path = match std::str::from_utf8(path_bytes) {
            Ok(s) => s,
            Err(_) => return MnftsResult::ErrIo,
        };

        let mut volume = match handle.inner.lock() {
            Ok(v) => v,
            Err(_) => return MnftsResult::ErrInternal,
        };

        // List parent directory and find the entry
        let (parent, name) = if path == "/" {
            ("/", "")
        } else {
            let trimmed = path.trim_end_matches('/');
            match trimmed.rfind('/') {
                Some(0) => ("/", &trimmed[1..]),
                Some(pos) => (&trimmed[..pos], &trimmed[pos + 1..]),
                None => ("/", trimmed),
            }
        };

        if name.is_empty() {
            // Root directory
            let info = unsafe { &mut *out };
            info.file_size = 0;
            info.is_directory = true;
            info.is_sparse = false;
            info.is_compressed = false;
            info.is_encrypted = false;
            info.mft_reference = 5; // Root MFT reference
            info.created_secs = -1;
            info.modified_secs = -1;
            info.accessed_secs = -1;
            return MnftsResult::Ok;
        }

        match list_directory(&mut volume, parent) {
            Ok(entries) => {
                if let Some(entry) = entries.iter().find(|e| e.name == name) {
                    let info = unsafe { &mut *out };
                    info.file_size = entry.file_size;
                    info.is_directory = entry.is_directory;
                    info.is_sparse = entry.is_sparse;
                    info.is_compressed = entry.is_compressed;
                    info.is_encrypted = entry.is_encrypted;
                    info.mft_reference = entry.mft_reference;
                    info.created_secs = system_time_to_secs(entry.created);
                    info.modified_secs = system_time_to_secs(entry.modified);
                    info.accessed_secs = system_time_to_secs(entry.accessed);
                    MnftsResult::Ok
                } else {
                    MnftsResult::ErrNotFound
                }
            }
            Err(e) => error_to_result(&e),
        }
    });

    result.unwrap_or(MnftsResult::ErrInternal)
}

// --- File reading ---

/// Read file data by MFT reference at given offset into caller-provided buffer.
/// Returns actual bytes read via `out_len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mnfts_read_file(
    handle: *mut MnftsVolumeHandle,
    mft_ref: u64,
    offset: u64,
    buf: *mut u8,
    buf_len: u32,
    out_len: *mut u32,
) -> MnftsResult {
    if handle.is_null() || buf.is_null() || out_len.is_null() {
        return MnftsResult::ErrNullPointer;
    }

    let result = std::panic::catch_unwind(|| {
        let handle = unsafe { &*handle };
        let mut volume = match handle.inner.lock() {
            Ok(v) => v,
            Err(_) => return MnftsResult::ErrInternal,
        };

        match read_file_range(&mut volume, mft_ref, offset, buf_len as usize) {
            Ok(data) => {
                let copy_len = data.len().min(buf_len as usize);
                unsafe {
                    std::ptr::copy_nonoverlapping(data.as_ptr(), buf, copy_len);
                    *out_len = copy_len as u32;
                }
                MnftsResult::Ok
            }
            Err(e) => error_to_result(&e),
        }
    });

    result.unwrap_or(MnftsResult::ErrInternal)
}
