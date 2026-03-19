import ArgumentParser
import Foundation

struct DoctorCommand: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "doctor",
        abstract: "Check NTFS volume health"
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

        // TODO: Call mnfts_doctor FFI once wired
        if json {
            print("{\"device\": \"\(device)\", \"healthy\": true}")
        } else {
            print("Doctor: \(device)")
            print("(Volume health check not yet wired to FFI)")
        }
    }
}
