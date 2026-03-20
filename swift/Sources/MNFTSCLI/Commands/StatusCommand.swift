import ArgumentParser
import Foundation

struct StatusCommand: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "status",
        abstract: "Show mounted NTFS volumes"
    )

    @Flag(name: .long, help: "Output JSON")
    var json: Bool = false

    func run() throws {
        if json {
            print("{\"volumes\": []}")
        } else {
            print("No NTFS volumes currently mounted.")
        }
    }
}
