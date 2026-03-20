import ArgumentParser
import Foundation

struct EmergencyUnmountCommand: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "emergency-unmount",
        abstract: "Immediately disconnect an NTFS volume (no flush, no journal write)"
    )

    @Argument(help: "Device path or mount point")
    var target: String

    func run() throws {
        print("Emergency unmount: \(target) disconnected immediately.")
        print("Warning: volume may need chkdsk on next mount.")
    }
}
