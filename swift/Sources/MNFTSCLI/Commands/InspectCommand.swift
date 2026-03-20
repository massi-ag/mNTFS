import ArgumentParser
import CLibMNFTS
import Foundation

struct InspectCommand: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "inspect",
        abstract: "Display raw NTFS volume metadata"
    )

    @Argument(help: "Device path or image file")
    var device: String

    @Flag(name: .long, help: "Output JSON")
    var json: Bool = false

    func run() throws {
        guard let fileHandle = FileHandle(forReadingAtPath: device) else {
            print("Error: Cannot open device or image: \(device)")
            throw ExitCode.failure
        }
        defer { fileHandle.closeFile() }

        let fd = fileHandle.fileDescriptor

        var info = mnfts_MnftsInspectInfo()
        var labelBuf = [UInt8](repeating: 0, count: 256)
        var labelLen: UInt32 = 0

        let result = mnfts_inspect(fd, &info, &labelBuf, UInt32(labelBuf.count), &labelLen)

        guard result == OK else {
            print("Error: mnfts_inspect failed with code \(result.rawValue)")
            throw ExitCode.failure
        }

        let volumeLabel: String
        if labelLen > 0 {
            volumeLabel = String(bytes: labelBuf.prefix(Int(labelLen)), encoding: .utf8) ?? ""
        } else {
            volumeLabel = ""
        }

        if json {
            printJSON(info: info, label: volumeLabel)
        } else {
            printTable(info: info, label: volumeLabel)
        }
    }

    private func printTable(info: mnfts_MnftsInspectInfo, label: String) {
        let version = "\(info.version_major).\(info.version_minor)"
        let serial = String(format: "0x%016llX", info.volume_serial)
        let dirtyText = info.dirty_flag ? "YES" : "no"

        print("NTFS Version:   \(version)")
        print("Volume Label:   \(label)")
        print("Serial Number:  \(serial)")
        print("Cluster Size:   \(info.cluster_size) bytes")
        print("Sector Size:    \(info.sector_size) bytes")
        print("Total Sectors:  \(info.total_sectors)")
        print("Dirty Flag:     \(dirtyText)")
    }

    private func printJSON(info: mnfts_MnftsInspectInfo, label: String) {
        let dict: [String: Any] = [
            "ntfs_version": "\(info.version_major).\(info.version_minor)",
            "volume_label": label,
            "volume_serial": String(format: "0x%016llX", info.volume_serial),
            "cluster_size": info.cluster_size,
            "sector_size": info.sector_size,
            "total_sectors": info.total_sectors,
            "dirty_flag": info.dirty_flag,
        ]
        if let data = try? JSONSerialization.data(
            withJSONObject: dict, options: [.prettyPrinted, .sortedKeys]
        ) {
            print(String(data: data, encoding: .utf8) ?? "{}")
        }
    }
}
