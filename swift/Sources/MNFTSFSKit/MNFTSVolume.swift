import Foundation

/// Represents a mounted NTFS volume.
/// Wraps the Rust volume handle and provides filesystem operations.
///
/// This class is used by MNFTSExtension (FSKit path) and can also
/// be used directly for testing without FSKit.
public final class MNFTSVolume: @unchecked Sendable {
    private let volumeHandle: UnsafeMutableRawPointer
    private let lock = NSLock()

    /// The volume label read from NTFS metadata.
    public let label: String

    /// Whether the volume passed health checks.
    public let isHealthy: Bool

    /// Open an NTFS volume from a file descriptor.
    /// Returns nil if the volume cannot be opened (not NTFS, corrupt, etc).
    public init?(fd: Int32) {
        guard let handle = MNFTSBridge.openVolume(fd: fd) else {
            return nil
        }
        self.volumeHandle = handle
        self.label = MNFTSBridge.volumeLabel(handle)
        self.isHealthy = MNFTSBridge.volumeIsHealthy(handle)
    }

    deinit {
        MNFTSBridge.closeVolume(volumeHandle)
    }

    /// List directory entries at the given path.
    public func listDirectory(path: String) -> [MNFTSBridge.DirEntry] {
        lock.lock()
        defer { lock.unlock() }
        return MNFTSBridge.readDirectory(volumeHandle, path: path)
    }

    /// Get file metadata at path.
    public func stat(path: String) -> MNFTSBridge.DirEntry? {
        lock.lock()
        defer { lock.unlock() }
        guard let info = MNFTSBridge.stat(volumeHandle, path: path) else { return nil }
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

    /// Read file data by MFT reference at offset.
    public func readFile(mftReference: UInt64, offset: UInt64, length: UInt32) -> Data? {
        lock.lock()
        defer { lock.unlock() }
        return MNFTSBridge.readFile(volumeHandle, mftReference: mftReference, offset: offset, length: length)
    }
}
