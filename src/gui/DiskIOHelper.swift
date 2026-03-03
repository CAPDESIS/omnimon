// DiskIOHelper.swift - Collect per-process disk I/O via proc_pid_rusage
// Usage: DiskIOHelper <pid1> <pid2> ... OR DiskIOHelper --stdin (reads PIDs from stdin)
// Output: JSON array of {pid, diskReadMB, diskWriteMB}

import Foundation
import Darwin

struct DiskIOEntry: Codable {
    let pid: Int
    let diskReadMB: Double
    let diskWriteMB: Double
}

func collectDiskIO(for pids: [pid_t]) -> [DiskIOEntry] {
    return pids.compactMap { pid in
        var usage = rusage_info_v4()
        let result = withUnsafeMutablePointer(to: &usage) { ptr in
            ptr.withMemoryRebound(to: rusage_info_t?.self, capacity: 1) { rebound in
                proc_pid_rusage(pid, RUSAGE_INFO_V4, rebound)
            }
        }
        guard result == 0 else { return nil }
        return DiskIOEntry(
            pid: Int(pid),
            diskReadMB: Double(usage.ri_diskio_bytesread) / 1_048_576.0,
            diskWriteMB: Double(usage.ri_diskio_byteswritten) / 1_048_576.0
        )
    }
}

// Parse PIDs from arguments or stdin
var pids: [pid_t] = []

let args = CommandLine.arguments.dropFirst()
if args.first == "--stdin" {
    while let line = readLine() {
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        if let pid = Int32(trimmed) {
            pids.append(pid)
        }
    }
} else {
    for arg in args {
        if let pid = Int32(arg) {
            pids.append(pid)
        }
    }
}

let entries = collectDiskIO(for: pids)
let encoder = JSONEncoder()
encoder.outputFormatting = .prettyPrinted
if let data = try? encoder.encode(entries),
   let json = String(data: data, encoding: .utf8) {
    print(json)
}
