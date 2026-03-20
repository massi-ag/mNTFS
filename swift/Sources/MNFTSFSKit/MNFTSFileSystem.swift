import Foundation

/// mNFTS filesystem module.
/// The FSKit system extension integration requires packaging as a
/// macOS System Extension with proper code signing and entitlements.
/// This module provides the core logic that the extension will call.
///
/// Architecture:
///   FSKit extension (System Extension)
///     -> MNFTSExtension (FSUnaryFileSystem subclass)
///       -> MNFTSVolume (volume operations)
///         -> MNFTSBridge / FFIBridge (Swift)
///           -> libmnfts (Rust, via C-ABI)
///             -> ntfs crate (NTFS parsing)
///
/// Current status: FFI bridge is fully wired. FSKit extension
/// packaging requires an Xcode project with System Extension target.

public final class MNFTSFileSystem: @unchecked Sendable {
    public static let name = "mNFTS"
    public static let version = "0.1.0"

    private var volume: MNFTSVolume?
    private let lock = NSLock()

    public init() {}

    deinit {
        close()
    }

    /// Open an NTFS volume from a file descriptor.
    public func open(fd: Int32) -> Bool {
        lock.lock()
        defer { lock.unlock() }

        volume = MNFTSVolume(fd: fd)
        return volume != nil
    }

    /// Close the volume.
    public func close() {
        lock.lock()
        defer { lock.unlock() }

        // MNFTSVolume's deinit calls MNFTSBridge.closeVolume
        volume = nil
    }

    /// Get the volume label.
    public func label() -> String {
        lock.lock()
        defer { lock.unlock() }
        return volume?.label ?? "NTFS"
    }

    /// Check if volume is healthy.
    public func isHealthy() -> Bool {
        lock.lock()
        defer { lock.unlock() }
        return volume?.isHealthy ?? false
    }

    /// List directory entries.
    public func listDirectory(path: String) -> [MNFTSBridge.DirEntry] {
        lock.lock()
        defer { lock.unlock() }
        return volume?.listDirectory(path: path) ?? []
    }

    /// Get file info at path.
    public func stat(path: String) -> MNFTSBridge.DirEntry? {
        lock.lock()
        defer { lock.unlock() }
        return volume?.stat(path: path)
    }

    /// Read file data by MFT reference.
    public func readFile(mftReference: UInt64, offset: UInt64, length: UInt32) -> Data? {
        lock.lock()
        defer { lock.unlock() }
        return volume?.readFile(mftReference: mftReference, offset: offset, length: length)
    }
}
