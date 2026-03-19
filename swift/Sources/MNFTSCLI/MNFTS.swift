import ArgumentParser

@main
struct MNFTS: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "mnfts",
        abstract: "Mount and manage NTFS volumes on macOS",
        version: "0.1.0",
        subcommands: [VersionCmd.self]
    )
}

struct VersionCmd: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "version",
        abstract: "Print version information"
    )

    func run() {
        print("mnfts 0.1.0")
    }
}
