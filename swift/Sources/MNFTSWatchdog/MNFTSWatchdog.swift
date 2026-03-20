import Foundation

@main
struct MNFTSWatchdog {
    static func main() {
        guard CommandLine.arguments.count > 1,
              let pid = pid_t(CommandLine.arguments[1]) else {
            print("Usage: mnfts-watchdog <extension-pid>")
            Foundation.exit(1)
        }

        print("mNFTS Watchdog: monitoring PID \(pid)")

        let monitor = ProcessMonitor(pid: pid) { crashedPid in
            print("mNFTS Watchdog: extension (PID \(crashedPid)) crashed!")

            // Write crash report
            CrashReporter.writeCrashReport(pid: crashedPid, timestamp: Date())

            print("mNFTS Watchdog: crash report written to ~/Library/Logs/mNFTS/")
            Foundation.exit(0)
        }

        monitor.start()

        // Keep the process running
        RunLoop.main.run()
    }
}
