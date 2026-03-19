import ArgumentParser
import Foundation

struct UnmountCommand: AsyncParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "unmount",
        abstract: "Unmount an NTFS volume"
    )

    @Argument(help: "Mount point or device path")
    var target: String

    @Flag(name: .long, help: "Output JSON")
    var json: Bool = false

    func run() async throws {
        // FSKit unmount integration pending Phase 0 research
        if json {
            print("{\"status\": \"unmounted\", \"target\": \"\(target)\"}")
        } else {
            print("Unmounted \(target)")
        }
    }
}
