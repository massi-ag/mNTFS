import Foundation

/// mNFTS filesystem module.
/// The FSKit system extension integration requires packaging as a
/// macOS System Extension with proper code signing and entitlements.
/// This module provides the core logic that the extension will call.
///
/// Architecture:
///   FSKit extension (System Extension)
///     -> MNFTSFileSystem (this module)
///       -> FFIBridge (Swift)
///         -> libmnfts (Rust, via C-ABI)
///           -> ntfs crate (NTFS parsing)
///
/// Current status: FFI bridge is fully wired. FSKit extension
/// packaging requires an Xcode project with System Extension target.

public final class MNFTSFileSystem: @unchecked Sendable {
    public static let name = "mNFTS"
    public static let version = "0.1.0"

    private var volumeHandle: UnsafeMutableRawPointer?
    private let lock = NSLock()

    public init() {}

    deinit {
        close()
    }

    /// Open an NTFS volume from a file descriptor.
    public func open(fd: Int32) -> Bool {
        lock.lock()
        defer { lock.unlock() }

        if volumeHandle != nil {
            close()
        }
        volumeHandle = MNFTSBridge.openVolume(fd: fd)
        return volumeHandle != nil
    }

    /// Close the volume.
    public func close() {
        lock.lock()
        defer { lock.unlock() }

        if let handle = volumeHandle {
            MNFTSBridge.closeVolume(handle)
            volumeHandle = nil
        }
    }

    /// Get the volume label.
    public func label() -> String {
        lock.lock()
        defer { lock.unlock() }
        guard let handle = volumeHandle else { return "NTFS" }
        return MNFTSBridge.volumeLabel(handle)
    }

    /// Check if volume is healthy.
    public func isHealthy() -> Bool {
        lock.lock()
        defer { lock.unlock() }
        guard let handle = volumeHandle else { return false }
        return MNFTSBridge.volumeIsHealthy(handle)
    }

    /// List directory entries.
    public func listDirectory(path: String) -> [MNFTSBridge.DirEntry] {
        lock.lock()
        defer { lock.unlock() }
        guard let handle = volumeHandle else { return [] }
        return MNFTSBridge.readDirectory(handle, path: path)
    }

    /// Get file info at path.
    public func stat(path: String) -> MNFTSBridge.DirEntry? {
        lock.lock()
        defer { lock.unlock() }
        guard let handle = volumeHandle else { return nil }
        guard let info = MNFTSBridge.stat(handle, path: path) else { return nil }
        return MNFTSBridge.DirEntry(
            name: String(path.split(separator: "/").last ?? ""),
            isDirectory: info.is_directory,
            fileSize: info.file_size,
            mftReference: info.mft_reference,
            createdSecs: info.created_secs,
            modifiedSecs: info.modified_secs,
            accessedSecs: info.accessed_secs
        )
    }

    /// Read file data by MFT reference.
    public func readFile(mftReference: UInt64, offset: UInt64, length: UInt32) -> Data? {
        lock.lock()
        defer { lock.unlock() }
        guard let handle = volumeHandle else { return nil }
        return MNFTSBridge.readFile(handle, mftReference: mftReference, offset: offset, length: length)
    }
}
