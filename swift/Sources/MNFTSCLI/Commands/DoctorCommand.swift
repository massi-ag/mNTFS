import ArgumentParser
import CLibMNFTS
import Foundation

/// Collects diagnostic messages from the doctor callback.
private final class DoctorContext {
    var messages: [String] = []
}

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
        guard let fileHandle = FileHandle(forReadingAtPath: device) else {
            print("Error: Cannot open device or image: \(device)")
            throw ExitCode.failure
        }
        defer { fileHandle.closeFile() }

        let fd = fileHandle.fileDescriptor

        var isHealthy = false
        let context = DoctorContext()
        let contextPtr = Unmanaged.passUnretained(context).toOpaque()

        let callback: mnfts_MnftsDoctorCallback = { rawCtx, msgPtr, msgLen in
            guard let rawCtx, let msgPtr else { return }
            let ctx = Unmanaged<DoctorContext>.fromOpaque(rawCtx).takeUnretainedValue()
            let message = String(
                bytes: UnsafeBufferPointer(start: msgPtr, count: Int(msgLen)),
                encoding: .utf8
            ) ?? ""
            ctx.messages.append(message)
        }

        let result = mnfts_doctor(fd, &isHealthy, callback, contextPtr)

        guard result == OK else {
            print("Error: mnfts_doctor failed with code \(result.rawValue)")
            throw ExitCode.failure
        }

        if json {
            printJSON(healthy: isHealthy, messages: context.messages)
        } else {
            printReport(healthy: isHealthy, messages: context.messages)
        }
    }

    private func printReport(healthy: Bool, messages: [String]) {
        let status = healthy ? "HEALTHY" : "UNHEALTHY"
        print("Volume Status: \(status)")
        if !messages.isEmpty {
            print("")
            print("Diagnostics:")
            for message in messages {
                print("  - \(message)")
            }
        }
    }

    private func printJSON(healthy: Bool, messages: [String]) {
        let dict: [String: Any] = [
            "healthy": healthy,
            "messages": messages,
        ]
        if let data = try? JSONSerialization.data(
            withJSONObject: dict, options: [.prettyPrinted, .sortedKeys]
        ) {
            print(String(data: data, encoding: .utf8) ?? "{}")
        }
    }
}
