import FSKit
import Foundation
import MNFTSFSKit

// MARK: - Extension Entry Point

@available(macOS 15.4, *)
@main
struct MNFTSFSExtension: UnaryFileSystemExtension {
    let fileSystem: FSUnaryFileSystem & FSUnaryFileSystemOperations = NTFSFileSystem()
}

// MARK: - Helpers

/// Open the block device's BSD name as a file descriptor for FFI.
@available(macOS 15.4, *)
func openBlockDevice(_ resource: FSBlockDeviceResource) -> Int32? {
    let path = "/dev/\(resource.bsdName)"
    let fd = open(path, O_RDONLY)
    return fd >= 0 ? fd : nil
}

/// Convert unix timestamp to timespec.
func timespecFromUnix(_ secs: Int64) -> timespec {
    timespec(tv_sec: Int(secs), tv_nsec: 0)
}

// MARK: - FSUnaryFileSystem

@available(macOS 15.4, *)
final class NTFSFileSystem: FSUnaryFileSystem, FSUnaryFileSystemOperations {

    func probeResource(resource: FSResource) async throws -> FSProbeResult {
        guard let blockDevice = resource as? FSBlockDeviceResource else {
            return .notRecognized
        }

        // Read boot sector to check "NTFS    " at offset 3
        var bootSector = Data(count: 512)
        let bytesRead = try bootSector.withUnsafeMutableBytes { ptr in
            try blockDevice.read(into: ptr, startingAt: 0, length: 512)
        }
        guard bytesRead >= 11 else { return .notRecognized }
        guard bootSector[3..<11] == Data("NTFS    ".utf8) else {
            return .notRecognized
        }

        // Open via FFI to read label
        guard let fd = openBlockDevice(blockDevice) else { return .notRecognized }
        defer { close(fd) }

        if let volume = MNFTSVolume(fd: fd) {
            return .usableButLimited(
                name: volume.label,
                containerID: FSContainerIdentifier(uuid: UUID())
            )
        }

        return .notRecognized
    }

    func loadResource(
        resource: FSResource,
        options: FSTaskOptions
    ) async throws -> FSVolume {
        guard let blockDevice = resource as? FSBlockDeviceResource else {
            throw POSIXError(.ENXIO)
        }

        guard let fd = openBlockDevice(blockDevice) else {
            throw POSIXError(.EACCES)
        }
        // Note: fd ownership transfers to MNFTSVolume (which dups it internally)

        guard let ntfsVolume = MNFTSVolume(fd: fd) else {
            close(fd)
            throw POSIXError(.EIO)
        }
        close(fd)

        let volume = NTFSFSVolume(ntfsVolume: ntfsVolume)
        containerStatus = .ready
        return volume
    }

    func unloadResource(
        resource: FSResource,
        options: FSTaskOptions
    ) async throws {
        containerStatus = .notReady(status: POSIXError(.ENXIO))
    }
}

// MARK: - FSVolume

@available(macOS 15.4, *)
final class NTFSFSVolume: FSVolume, FSVolume.Operations, FSVolume.PathConfOperations,
    FSVolume.ReadWriteOperations, @unchecked Sendable
{
    let ntfs: MNFTSVolume
    private var rootItem: NTFSItem?

    init(ntfsVolume: MNFTSVolume) {
        self.ntfs = ntfsVolume
        super.init(
            volumeID: FSVolume.Identifier(),
            volumeName: FSFileName(string: ntfsVolume.label)
        )
    }

    // MARK: PathConfOperations

    var supportedVolumeCapabilities: FSVolume.SupportedCapabilities {
        let caps = SupportedCapabilities()
        caps.caseFormat = .insensitive
        caps.supportsHardLinks = false
        caps.supportsSymbolicLinks = false
        caps.supportsJournal = false
        caps.supportsActiveJournal = false
        caps.supportsPersistentObjectIDs = true
        caps.supports64BitObjectIDs = true
        caps.supportsSparseFiles = true
        caps.supportsZeroRuns = true
        caps.supports2TBFiles = true
        caps.supportsHiddenFiles = true
        caps.supportsFastStatFS = true
        caps.doesNotSupportRootTimes = false
        caps.doesNotSupportVolumeSizes = false
        caps.doesNotSupportImmutableFiles = true
        caps.doesNotSupportSettingFilePermissions = true
        return caps
    }

    var volumeStatistics: FSStatFSResult {
        let stats = FSStatFSResult(fileSystemTypeName: "mntfs")
        stats.blockSize = 4096
        stats.ioSize = 65536
        return stats
    }

    var maximumLinkCount: Int { 1 }
    var maximumNameLength: Int { 255 }
    var maximumFileSizeInBits: Int { 64 }
    var maximumXattrSize: Int { 0 }
    var restrictsOwnershipChanges: Bool { true }
    var truncatesLongNames: Bool { false }

    // MARK: Lifecycle

    func activate(options: FSTaskOptions) async throws -> FSItem {
        let root = NTFSItem(
            name: "/", mftReference: 5,
            isDirectory: true, fileSize: 0, path: "/"
        )
        root.attrs.fileID = .rootDirectory
        root.attrs.parentID = .parentOfRoot
        root.attrs.uid = 0
        root.attrs.gid = 0
        root.attrs.type = .directory
        root.attrs.mode = UInt32(S_IFDIR | 0o755)
        root.attrs.linkCount = 1
        self.rootItem = root
        return root
    }

    func deactivate(options: FSDeactivateOptions) async throws {
        rootItem = nil
    }

    func mount(options: FSTaskOptions) async throws { }
    func unmount() async { }
    func synchronize(flags: FSSyncFlags) async throws { }

    // MARK: Attributes

    func attributes(
        _ desiredAttributes: FSItem.GetAttributesRequest,
        of item: FSItem
    ) async throws -> FSItem.Attributes {
        guard let ntfsItem = item as? NTFSItem else { throw POSIXError(.ENOENT) }
        return ntfsItem.attrs
    }

    func setAttributes(
        _ newAttributes: FSItem.SetAttributesRequest,
        on item: FSItem
    ) async throws -> FSItem.Attributes {
        throw POSIXError(.EROFS)
    }

    // MARK: Lookup

    func lookupItem(
        named name: FSFileName,
        inDirectory directory: FSItem
    ) async throws -> (FSItem, FSFileName) {
        guard let dir = directory as? NTFSItem, dir.isDirectory else {
            throw POSIXError(.ENOTDIR)
        }

        let nameStr = name.string ?? ""
        let entries = ntfs.listDirectory(path: dir.path)
        for entry in entries {
            if entry.name.caseInsensitiveCompare(nameStr) == .orderedSame {
                let childPath = dir.path == "/"
                    ? "/\(entry.name)" : "\(dir.path)/\(entry.name)"
                let child = NTFSItem(
                    name: entry.name, mftReference: entry.mftReference,
                    isDirectory: entry.isDirectory, fileSize: entry.fileSize,
                    path: childPath
                )
                child.applyTimestamps(entry)
                return (child, FSFileName(string: entry.name))
            }
        }
        throw POSIXError(.ENOENT)
    }

    // MARK: Enumerate

    func enumerateDirectory(
        _ directory: FSItem,
        startingAt cookie: FSDirectoryCookie,
        verifier: FSDirectoryVerifier,
        attributes desiredAttributes: FSItem.GetAttributesRequest?,
        packer: FSDirectoryEntryPacker
    ) async throws -> FSDirectoryVerifier {
        guard let dir = directory as? NTFSItem, dir.isDirectory else {
            throw POSIXError(.ENOTDIR)
        }

        let entries = ntfs.listDirectory(path: dir.path)
        let startIndex = cookie == .initial ? 0 : Int(cookie.rawValue)

        for i in startIndex..<entries.count {
            let entry = entries[i]
            let childPath = dir.path == "/"
                ? "/\(entry.name)" : "\(dir.path)/\(entry.name)"
            let child = NTFSItem(
                name: entry.name, mftReference: entry.mftReference,
                isDirectory: entry.isDirectory, fileSize: entry.fileSize,
                path: childPath
            )
            child.applyTimestamps(entry)

            let packed = packer.packEntry(
                name: FSFileName(string: entry.name),
                itemType: entry.isDirectory ? .directory : .file,
                itemID: FSItem.Identifier(rawValue: entry.mftReference) ?? .invalid,
                nextCookie: FSDirectoryCookie(rawValue: UInt64(i + 1)),
                attributes: desiredAttributes != nil ? child.attrs : nil
            )
            if !packed { break }
        }

        return FSDirectoryVerifier(rawValue: 0)
    }

    // MARK: Read

    func read(
        from item: FSItem,
        at offset: off_t,
        length: Int,
        into buffer: FSMutableFileDataBuffer
    ) async throws -> Int {
        guard let ntfsItem = item as? NTFSItem, !ntfsItem.isDirectory else {
            throw POSIXError(.EISDIR)
        }

        guard let data = ntfs.readFile(
            mftReference: ntfsItem.mftReference,
            offset: UInt64(offset),
            length: UInt32(min(length, Int(UInt32.max)))
        ) else {
            throw POSIXError(.EIO)
        }

        data.withUnsafeBytes { src in
            buffer.withUnsafeMutableBytes { dst in
                let count = min(src.count, dst.count)
                if count > 0 {
                    dst.baseAddress!.copyMemory(from: src.baseAddress!, byteCount: count)
                }
            }
        }
        return data.count
    }

    func write(contents: Data, to item: FSItem, at offset: off_t) async throws -> Int {
        throw POSIXError(.EROFS)
    }

    // MARK: Mutating ops (read-only)

    func createItem(
        named name: FSFileName, type: FSItem.ItemType,
        inDirectory directory: FSItem, attributes: FSItem.SetAttributesRequest
    ) async throws -> (FSItem, FSFileName) { throw POSIXError(.EROFS) }

    func createSymbolicLink(
        named name: FSFileName, inDirectory directory: FSItem,
        attributes: FSItem.SetAttributesRequest, linkContents contents: FSFileName
    ) async throws -> (FSItem, FSFileName) { throw POSIXError(.EROFS) }

    func createLink(
        to item: FSItem, named name: FSFileName, inDirectory directory: FSItem
    ) async throws -> FSFileName { throw POSIXError(.EROFS) }

    func removeItem(
        _ item: FSItem, named name: FSFileName, fromDirectory directory: FSItem
    ) async throws { throw POSIXError(.EROFS) }

    func renameItem(
        _ item: FSItem, inDirectory sourceDirectory: FSItem,
        named sourceName: FSFileName, to destinationName: FSFileName,
        inDirectory destinationDirectory: FSItem, overItem: FSItem?
    ) async throws -> FSFileName { throw POSIXError(.EROFS) }

    func readSymbolicLink(_ item: FSItem) async throws -> FSFileName {
        throw POSIXError(.EINVAL)
    }

    func reclaimItem(_ item: FSItem) async throws { }
}

// MARK: - NTFSItem

@available(macOS 15.4, *)
final class NTFSItem: FSItem, @unchecked Sendable {
    let itemName: String
    let mftReference: UInt64
    let isDirectory: Bool
    let fileSize: UInt64
    let path: String
    let attrs: FSItem.Attributes

    init(name: String, mftReference: UInt64, isDirectory: Bool, fileSize: UInt64, path: String) {
        self.itemName = name
        self.mftReference = mftReference
        self.isDirectory = isDirectory
        self.fileSize = fileSize
        self.path = path

        let a = FSItem.Attributes()
        a.fileID = FSItem.Identifier(rawValue: mftReference) ?? .invalid
        a.uid = 99
        a.gid = 99
        a.type = isDirectory ? .directory : .file
        a.mode = isDirectory ? UInt32(S_IFDIR | 0o755) : UInt32(S_IFREG | 0o644)
        a.linkCount = 1
        a.size = fileSize
        a.allocSize = (fileSize + 4095) / 4096 * 4096
        self.attrs = a
        super.init()
    }

    func applyTimestamps(_ entry: MNFTSBridge.DirEntry) {
        if entry.createdSecs > 0 {
            attrs.birthTime = timespecFromUnix(entry.createdSecs)
        }
        if entry.modifiedSecs > 0 {
            let ts = timespecFromUnix(entry.modifiedSecs)
            attrs.modifyTime = ts
            attrs.changeTime = ts
        }
        if entry.accessedSecs > 0 {
            attrs.accessTime = timespecFromUnix(entry.accessedSecs)
        }
    }
}
