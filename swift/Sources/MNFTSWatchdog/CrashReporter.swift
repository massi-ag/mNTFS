import Foundation

enum CrashReporter {
    static let logDirectory = FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent("Library/Logs/mNFTS")

    static func writeCrashReport(pid: pid_t, timestamp: Date) {
        try? FileManager.default.createDirectory(
            at: logDirectory, withIntermediateDirectories: true
        )

        let formatter = ISO8601DateFormatter()
        let dateStr = formatter.string(from: timestamp)
        let filename = "crash-\(dateStr).log"
        let fileURL = logDirectory.appendingPathComponent(filename)

        let report = """
        mNFTS Crash Report
        Timestamp: \(dateStr)
        Extension PID: \(pid)
        Status: FSKit extension process exited unexpectedly.
        Action: Volume was force-unmounted for safety.
        """

        try? report.write(to: fileURL, atomically: true, encoding: .utf8)
    }
}
