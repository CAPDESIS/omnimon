import Foundation

final class TelemetryRecorder {
    static let shared = TelemetryRecorder()

    private let historyPath: String
    private let maxBytes: UInt64 = 5 * 1024 * 1024  // 5 MB
    private let queue = DispatchQueue(label: "com.macmon.telemetry", qos: .utility)

    private init() {
        let configDir = NSString(string: "~/.config/macmon").expandingTildeInPath
        historyPath = (configDir as NSString).appendingPathComponent("history.jsonl")
    }

    func recordKill(pid: Int, name: String, ramMB: Double, reason: String) {
        queue.async { [weak self] in
            guard let self = self else { return }

            let formatter = ISO8601DateFormatter()
            formatter.formatOptions = [.withInternetDateTime]
            let ts = formatter.string(from: Date())

            let entry: [String: Any] = [
                "ts": ts,
                "action": "kill",
                "pid": pid,
                "name": name,
                "ramMB": ramMB,
                "reason": reason,
            ]

            guard let data = try? JSONSerialization.data(withJSONObject: entry, options: []),
                  var line = String(data: data, encoding: .utf8) else { return }
            line += "\n"

            self.rotateIfNeeded()

            let dir = (self.historyPath as NSString).deletingLastPathComponent
            FileManager.default.createFile(atPath: self.historyPath, contents: nil, attributes: nil)
            try? FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true, attributes: nil)

            if let handle = FileHandle(forWritingAtPath: self.historyPath) {
                handle.seekToEndOfFile()
                if let lineData = line.data(using: .utf8) {
                    handle.write(lineData)
                }
                handle.closeFile()
            }
        }
    }

    private func rotateIfNeeded() {
        guard let attrs = try? FileManager.default.attributesOfItem(atPath: historyPath),
              let size = attrs[.size] as? UInt64,
              size > maxBytes else { return }

        let backup = historyPath + ".1"
        try? FileManager.default.removeItem(atPath: backup)
        try? FileManager.default.moveItem(atPath: historyPath, toPath: backup)
    }
}
