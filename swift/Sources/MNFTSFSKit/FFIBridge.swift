import CLibMNFTS
import Foundation

/// Swift wrapper around C-ABI functions from libmnfts.
public enum MNFTSBridge {
    static func ping() -> Bool {
        return mnfts_ping() == OK
    }

    static func version() -> String {
        guard let cStr = mnfts_version() else { return "unknown" }
        return String(cString: cStr)
    }

    // The C header declares MnftsVolumeHandle as an opaque struct.
    // Swift imports pointers to it as OpaquePointer via the C module.
    // We store it as UnsafeMutableRawPointer for easy casting.

    /// Open an NTFS volume from a file descriptor.
    static func openVolume(fd: Int32) -> UnsafeMutableRawPointer? {
        return UnsafeMutableRawPointer(mnfts_open_volume(fd))
    }

    /// Close a volume handle.
    static func closeVolume(_ handle: UnsafeMutableRawPointer) {
        mnfts_close_volume(OpaquePointer(handle))
    }

    /// Get the volume label.
    static func volumeLabel(_ handle: UnsafeMutableRawPointer) -> String {
        var buf = [UInt8](repeating: 0, count: 256)
        let len = mnfts_volume_label(
            OpaquePointer(handle),
            &buf,
            UInt32(buf.count)
        )
        if len == 0 { return "NTFS" }
        return String(bytes: buf[..<Int(len)], encoding: .utf8) ?? "NTFS"
    }

    /// Check if the volume is healthy.
    static func volumeIsHealthy(_ handle: UnsafeMutableRawPointer) -> Bool {
        return mnfts_volume_is_healthy(OpaquePointer(handle))
    }

    /// Directory entry from readdir callback.
    public struct DirEntry: Sendable {
        public let name: String
        public let isDirectory: Bool
        public let fileSize: UInt64
        public let mftReference: UInt64
        public let createdSecs: Int64
        public let modifiedSecs: Int64
        public let accessedSecs: Int64
    }

    /// List directory entries at the given path.
    static func readDirectory(_ handle: UnsafeMutableRawPointer, path: String) -> [DirEntry] {
        let entries = NSMutableArray()
        let context = Unmanaged.passUnretained(entries).toOpaque()

        let callback: mnfts_MnftsDirEntryCallback = { ctx, namePtr, nameLen, isDir, fileSize, mftRef, created, modified, accessed in
            guard let ctx, let namePtr else { return }
            let arr = Unmanaged<NSMutableArray>.fromOpaque(ctx).takeUnretainedValue()
            let name = String(
                bytes: UnsafeBufferPointer(start: namePtr, count: Int(nameLen)),
                encoding: .utf8
            ) ?? ""
            let entry = DirEntry(
                name: name,
                isDirectory: isDir,
                fileSize: fileSize,
                mftReference: mftRef,
                createdSecs: created,
                modifiedSecs: modified,
                accessedSecs: accessed
            )
            arr.add(entry)
        }

        let pathBytes = Array(path.utf8)
        pathBytes.withUnsafeBufferPointer { buf in
            _ = mnfts_readdir(
                OpaquePointer(handle),
                buf.baseAddress,
                UInt32(buf.count),
                callback,
                context
            )
        }

        return entries.compactMap { $0 as? DirEntry }
    }

    /// Get file metadata at a path.
    static func stat(_ handle: UnsafeMutableRawPointer, path: String) -> mnfts_MnftsFileInfo? {
        var info = mnfts_MnftsFileInfo()
        let pathBytes = Array(path.utf8)
        let result = pathBytes.withUnsafeBufferPointer { buf in
            mnfts_stat(
                OpaquePointer(handle),
                buf.baseAddress,
                UInt32(buf.count),
                &info
            )
        }
        return result == OK ? info : nil
    }

    /// Read file data by MFT reference at offset.
    static func readFile(
        _ handle: UnsafeMutableRawPointer,
        mftReference: UInt64,
        offset: UInt64,
        length: UInt32
    ) -> Data? {
        var buf = [UInt8](repeating: 0, count: Int(length))
        var outLen: UInt32 = 0
        let result = mnfts_read_file(
            OpaquePointer(handle),
            mftReference,
            offset,
            &buf,
            length,
            &outLen
        )
        guard result == OK else { return nil }
        return Data(buf[..<Int(outLen)])
    }
}
