import ArgumentParser

@main
struct MNFTS: AsyncParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "mnfts",
        abstract: "Mount and manage NTFS volumes on macOS",
        version: "0.1.0",
        subcommands: [
            MountCommand.self,
            UnmountCommand.self,
            EmergencyUnmountCommand.self,
            StatusCommand.self,
            InspectCommand.self,
            DoctorCommand.self,
        ]
    )
}
