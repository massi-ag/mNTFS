import Foundation

/// Monitors a process by PID using kqueue/EVFILT_PROC.
/// Calls the handler when the process exits.
final class ProcessMonitor {
    private let pid: pid_t
    private let onExit: (pid_t) -> Void
    private var source: DispatchSourceProcess?

    init(pid: pid_t, onExit: @escaping (pid_t) -> Void) {
        self.pid = pid
        self.onExit = onExit
    }

    func start() {
        let source = DispatchSource.makeProcessSource(
            identifier: pid,
            eventMask: .exit,
            queue: .main
        )
        source.setEventHandler { [weak self] in
            guard let self else { return }
            self.onExit(self.pid)
        }
        source.resume()
        self.source = source
    }

    func stop() {
        source?.cancel()
        source = nil
    }
}
