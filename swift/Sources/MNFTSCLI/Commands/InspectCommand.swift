import ArgumentParser
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
        guard FileManager.default.fileExists(atPath: device) else {
            print("Error: Device or image not found: \(device)")
            throw ExitCode.failure
        }

        // TODO: Call mnfts_inspect FFI once wired
        if json {
            print("{\"device\": \"\(device)\", \"status\": \"not yet implemented\"}")
        } else {
            print("Inspect: \(device)")
            print("(Volume inspection not yet wired to FFI)")
        }
    }
}
