import Foundation
import Darwin

/// In-process kill logic for ProcessPicker standalone mode.
/// Reuses AIService.immutableProtectedProcessNames for the safety blocklist.
final class ProcessKiller {
    static let shared = ProcessKiller()

    private let gracePeriod: UInt32 = 3  // seconds between SIGTERM and SIGKILL

    private func killerLog(_ message: String) {
        let logDir = NSHomeDirectory() + "/.local/log/macmon"
        let logFile = logDir + "/process-picker.log"
        let fm = FileManager.default
        try? fm.createDirectory(atPath: logDir, withIntermediateDirectories: true)
        if !fm.fileExists(atPath: logFile) {
            fm.createFile(atPath: logFile, contents: nil)
        }
        let line = "\(ISO8601DateFormatter().string(from: Date())) [ProcessKiller] \(message)\n"
        if let data = line.data(using: .utf8), let handle = FileHandle(forWritingAtPath: logFile) {
            handle.seekToEndOfFile()
            handle.write(data)
            try? handle.close()
        }
    }

    /// Resolve the path to graceful-quit.sh relative to MACMON_HOME or the binary location.
    private func gracefulQuitPath() -> String? {
        // Try MACMON_HOME first
        if let home = ProcessInfo.processInfo.environment["MACMON_HOME"] {
            let path = home + "/scripts/graceful-quit.sh"
            if FileManager.default.isExecutableFile(atPath: path) { return path }
        }
        // Try relative to the binary's bundle (for .app)
        let execPath = ProcessInfo.processInfo.arguments[0]
        let execDir = (execPath as NSString).deletingLastPathComponent
        // Binary in Contents/Helpers, scripts in Contents/SharedSupport/scripts
        let bundlePath = (execDir as NSString).deletingLastPathComponent + "/SharedSupport/scripts/graceful-quit.sh"
        if FileManager.default.isExecutableFile(atPath: bundlePath) { return bundlePath }
        // Fallback: relative to binary itself (flat layout)
        let siblingPath = execDir + "/scripts/graceful-quit.sh"
        if FileManager.default.isExecutableFile(atPath: siblingPath) { return siblingPath }
        // Try ~/.local/libexec/macmon
        let installPath = NSHomeDirectory() + "/.local/libexec/macmon/scripts/graceful-quit.sh"
        if FileManager.default.isExecutableFile(atPath: installPath) { return installPath }
        return nil
    }

    /// Check if a process name is in the immutable protected blocklist.
    private func isProtected(_ name: String) -> Bool {
        return AIService.immutableProtectedProcessNames.contains(name)
    }

    /// Check if a PID is still alive.
    private func isAlive(_ pid: Int) -> Bool {
        return kill(pid_t(pid), 0) == 0
    }

    /// Run graceful-quit.sh with given arguments.
    private func runGracefulQuit(args: [String]) {
        guard let script = gracefulQuitPath() else {
            killerLog("graceful-quit.sh not found, skipping graceful quit for \(args)")
            return
        }
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/bin/bash")
        process.arguments = [script] + args
        process.standardOutput = FileHandle.nullDevice
        process.standardError = FileHandle.nullDevice
        do {
            try process.run()
            process.waitUntilExit()
        } catch {
            killerLog("graceful-quit.sh failed: \(error)")
        }
    }

    /// Kill a list of ProcessEntry items in-process. Returns the count of
    /// successfully killed processes. Runs on a background queue and calls
    /// completion on the main thread.
    func killProcesses(_ entries: [ProcessEntry], completion: @escaping (Int, [Int]) -> Void) {
        DispatchQueue.global(qos: .userInitiated).async { [self] in
            var killedPIDs: [Int] = []
            var termPIDs: [(pid: Int, name: String)] = []

            for entry in entries {
                let pid = entry.pid
                let name = entry.name
                let execName = entry.execName

                // Safety checks
                guard pid > 1 else {
                    killerLog("BLOCKED: refusing to kill PID \(pid) (too low)")
                    continue
                }
                if isProtected(execName) || isProtected(name) {
                    killerLog("BLOCKED: refusing to kill protected process \(execName) (PID \(pid))")
                    continue
                }
                guard isAlive(pid) else {
                    killerLog("SKIP: PID \(pid) (\(name)) already dead")
                    killedPIDs.append(pid)
                    continue
                }

                // Chrome tabs: delegate to graceful-quit.sh
                if name == "Chrome Tab" {
                    killerLog("Closing Chrome tab PID \(pid) via graceful-quit.sh")
                    let url = entry.cwd  // cwd holds the tab URL for Chrome tabs
                    if url.isEmpty {
                        runGracefulQuit(args: ["chrome-tab", String(pid)])
                    } else {
                        runGracefulQuit(args: ["chrome-tab", String(pid), url])
                    }
                    // Check if it died
                    usleep(500_000)  // 0.5s for Chrome to close the tab
                    if !isAlive(pid) {
                        killedPIDs.append(pid)
                    }
                    continue
                }

                // .app processes: try graceful quit via AppleScript
                if !execName.contains("/") && execName != "node" && !execName.hasPrefix("node") {
                    killerLog("Attempting graceful quit for app \(execName) (PID \(pid))")
                    runGracefulQuit(args: ["app", execName])
                    termPIDs.append((pid: pid, name: name))
                    continue
                }

                // Default: SIGTERM
                killerLog("Sending SIGTERM to \(name) (PID \(pid))")
                kill(pid_t(pid), SIGTERM)
                termPIDs.append((pid: pid, name: name))
            }

            // Wait grace period for SIGTERM targets
            if !termPIDs.isEmpty {
                sleep(gracePeriod)
            }

            // SIGKILL stragglers
            for (pid, name) in termPIDs {
                if isAlive(pid) {
                    // Re-check protection before SIGKILL
                    if isProtected(name) {
                        killerLog("BLOCKED: refusing SIGKILL for protected process \(name) (PID \(pid))")
                        continue
                    }
                    killerLog("Sending SIGKILL to \(name) (PID \(pid))")
                    kill(pid_t(pid), SIGKILL)
                }
                killedPIDs.append(pid)
            }

            let count = killedPIDs.count
            killerLog("Kill complete: \(count)/\(entries.count) processes handled")

            DispatchQueue.main.async {
                completion(count, killedPIDs)
            }
        }
    }
}
