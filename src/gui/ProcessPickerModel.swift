import Foundation

// MARK: - Data Model

struct ProcessEntry: Codable {
    var pid: Int
    var name: String
    var execName: String
    var ramMB: Double
    var cpuPct: Double
    var uptime: String
    var uptimeSeconds: Int
    var cwd: String
    var tty: String
    var idle: Bool
    var detail: String
    var group: String
    var isSystem: Bool
    var state: String
    var diskReadMB: Double
    var diskWriteMB: Double

    var selected: Bool = false

    enum CodingKeys: String, CodingKey {
        case pid, name, execName, ramMB, cpuPct, uptime, uptimeSeconds, cwd, tty, idle, detail, group, isSystem, state, diskReadMB, diskWriteMB
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        pid = try c.decode(Int.self, forKey: .pid)
        name = try c.decode(String.self, forKey: .name)
        execName = try c.decodeIfPresent(String.self, forKey: .execName) ?? name
        ramMB = try c.decode(Double.self, forKey: .ramMB)
        cpuPct = try c.decode(Double.self, forKey: .cpuPct)
        uptime = try c.decode(String.self, forKey: .uptime)
        uptimeSeconds = try c.decode(Int.self, forKey: .uptimeSeconds)
        cwd = try c.decode(String.self, forKey: .cwd)
        tty = try c.decode(String.self, forKey: .tty)
        idle = try c.decode(Bool.self, forKey: .idle)
        detail = try c.decode(String.self, forKey: .detail)
        group = try c.decode(String.self, forKey: .group)
        isSystem = try c.decode(Bool.self, forKey: .isSystem)
        state = try c.decode(String.self, forKey: .state)
        diskReadMB = try c.decodeIfPresent(Double.self, forKey: .diskReadMB) ?? 0
        diskWriteMB = try c.decodeIfPresent(Double.self, forKey: .diskWriteMB) ?? 0
    }

    init(pid: Int, name: String, execName: String = "", ramMB: Double, cpuPct: Double, uptime: String = "", uptimeSeconds: Int = 0, cwd: String = "", tty: String = "", idle: Bool = false, detail: String = "", group: String = "", isSystem: Bool = false, state: String = "", diskReadMB: Double = 0, diskWriteMB: Double = 0) {
        self.pid = pid
        self.name = name
        self.execName = execName.isEmpty ? name : execName
        self.ramMB = ramMB
        self.cpuPct = cpuPct
        self.uptime = uptime
        self.uptimeSeconds = uptimeSeconds
        self.cwd = cwd
        self.tty = tty
        self.idle = idle
        self.detail = detail
        self.group = group
        self.isSystem = isSystem
        self.state = state
        self.diskReadMB = diskReadMB
        self.diskWriteMB = diskWriteMB
    }
}

struct SystemHealth: Codable {
    let freePercent: Int
    let swapUsedMB: Int
    let totalProcesses: Int
    let idleCount: Int
    let monitoredCount: Int
    let physMemGB: Double
}

struct ProcessData: Codable {
    let processes: [ProcessEntry]
    let system: SystemHealth
}

// MARK: - Sort Configuration

enum SortColumn: String {
    case name, ramMB, cpuPct, uptime, pid, tty, cwd, detail, idle, group, state, diskReadMB, diskWriteMB
}

enum SortOrder {
    case ascending, descending
    mutating func toggle() {
        self = (self == .ascending) ? .descending : .ascending
    }
}

// MARK: - Process Group

struct ProcessGroup {
    let name: String
    var entries: [Int]  // indices into the flat list
    var collapsed: Bool = false

    var count: Int { return entries.count }
}

// MARK: - View Model

class ProcessViewModel {
    var allProcesses: [ProcessEntry] = []
    var filteredIndices: [Int] = []
    var groups: [String: ProcessGroup] = [:]
    var groupOrder: [String] = []
    var groupingEnabled: Bool = true

    // Display rows: either flat indices or group headers + entries
    var displayRows: [DisplayRow] = []

    var sortColumn: SortColumn = .name
    var sortOrder: SortOrder = .ascending
    var searchText: String = ""
    var hideSystemProcesses: Bool = false
    var showOnlyIdle: Bool = false

    enum DisplayRow {
        case groupHeader(String, Int, Double, Bool)  // name, count, totalRAM, collapsed
        case process(Int)  // index into allProcesses
    }

    func load(from data: ProcessData) {
        allProcesses = data.processes
        applyFilterAndSort()
    }

    func applyFilterAndSort() {
        // Filter
        let search = searchText.lowercased()
        if search.isEmpty {
            filteredIndices = Array(0..<allProcesses.count)
        } else {
            filteredIndices = allProcesses.indices.filter { i in
                let p = allProcesses[i]
                return p.name.lowercased().contains(search) ||
                       String(p.pid).contains(search) ||
                       p.detail.lowercased().contains(search) ||
                       p.cwd.lowercased().contains(search) ||
                       p.group.lowercased().contains(search)
            }
        }

        if hideSystemProcesses {
            filteredIndices = filteredIndices.filter { !allProcesses[$0].isSystem }
        }

        if showOnlyIdle {
            filteredIndices = filteredIndices.filter { allProcesses[$0].idle }
        }

        // Sort
        filteredIndices.sort { a, b in
            let pa = allProcesses[a]
            let pb = allProcesses[b]
            let result: Bool
            switch sortColumn {
            case .name:     result = pa.name.localizedCaseInsensitiveCompare(pb.name) == .orderedAscending
            case .ramMB:    result = pa.ramMB < pb.ramMB
            case .cpuPct:   result = pa.cpuPct < pb.cpuPct
            case .uptime:   result = pa.uptimeSeconds < pb.uptimeSeconds
            case .pid:      result = pa.pid < pb.pid
            case .tty:      result = pa.tty < pb.tty
            case .cwd:      result = pa.cwd < pb.cwd
            case .detail:   result = pa.detail < pb.detail
            case .idle:     result = !pa.idle && pb.idle
            case .group:    result = pa.group < pb.group
            case .state:    result = pa.state < pb.state
            case .diskReadMB:  result = pa.diskReadMB < pb.diskReadMB
            case .diskWriteMB: result = pa.diskWriteMB < pb.diskWriteMB
            }
            return sortOrder == .ascending ? result : !result
        }

        // Build display rows
        buildDisplayRows()
    }

    private func buildDisplayRows() {
        displayRows.removeAll()

        let searching = !searchText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        if !groupingEnabled || searching {
            displayRows = filteredIndices.map { .process($0) }
            return
        }

        // Group by group field
        var groupMap: [String: [Int]] = [:]
        var seenOrder: [String] = []
        for idx in filteredIndices {
            let g = allProcesses[idx].group.isEmpty ? L("group.other") : allProcesses[idx].group
            if groupMap[g] == nil { seenOrder.append(g) }
            groupMap[g, default: []].append(idx)
        }

        for gName in seenOrder {
            guard let indices = groupMap[gName] else { continue }
            let totalRAM = indices.reduce(0.0) { $0 + allProcesses[$1].ramMB }
            let isCollapsed = groups[gName]?.collapsed ?? false
            groups[gName] = ProcessGroup(name: gName, entries: indices, collapsed: isCollapsed)
            displayRows.append(.groupHeader(gName, indices.count, totalRAM, isCollapsed))
            if !isCollapsed {
                for idx in indices {
                    displayRows.append(.process(idx))
                }
            }
        }
        groupOrder = seenOrder
    }

    func toggleGroup(_ name: String) {
        groups[name]?.collapsed.toggle()
        buildDisplayRows()
    }

    var selectedCount: Int {
        return allProcesses.filter { $0.selected }.count
    }

    var selectedRAM: Double {
        return allProcesses.filter { $0.selected }.reduce(0) { $0 + $1.ramMB }
    }

    var selectedPIDs: [Int] {
        return allProcesses.filter { $0.selected }.map { $0.pid }
    }
}

// MARK: - Config Assistant

struct ConfigQuickSettings {
    var ramFreePercent: Int = 25
    var swapUsedMB: Int = 2048
    var processMinRAMKB: Int = 102400
    var idleCPUPercent: Double = 1.0
    var checkIntervalSec: Int = 60
    var idleCheckSec: Int = 600
    var cooldownSec: Int = 300
    var killGraceSec: Int = 3
    var collectDiskIO: Bool = true

    static func parse(from yaml: String) -> ConfigQuickSettings {
        var s = ConfigQuickSettings()
        func intVal(_ pattern: String, _ fallback: Int) -> Int {
            guard let r = yaml.range(of: pattern, options: .regularExpression) else { return fallback }
            let token = String(yaml[r]).components(separatedBy: CharacterSet.decimalDigits.inverted).joined()
            return Int(token) ?? fallback
        }
        func doubleVal(_ pattern: String, _ fallback: Double) -> Double {
            guard let r = yaml.range(of: pattern, options: .regularExpression) else { return fallback }
            let m = String(yaml[r])
            let v = m.replacingOccurrences(of: "[^0-9.]", with: "", options: .regularExpression)
            return Double(v) ?? fallback
        }
        func boolVal(_ pattern: String, _ fallback: Bool) -> Bool {
            guard let r = yaml.range(of: pattern, options: .regularExpression) else { return fallback }
            let m = String(yaml[r]).lowercased()
            return m.contains("true")
        }

        s.ramFreePercent = intVal("ram_free_percent\\s*:\\s*[0-9]+", s.ramFreePercent)
        s.swapUsedMB = intVal("swap_used_mb\\s*:\\s*[0-9]+", s.swapUsedMB)
        s.processMinRAMKB = intVal("process_ram_min_kb\\s*:\\s*[0-9]+", s.processMinRAMKB)
        s.idleCPUPercent = doubleVal("idle_cpu_percent\\s*:\\s*[0-9]+(?:\\.[0-9]+)?", s.idleCPUPercent)
        s.checkIntervalSec = intVal("check\\s*:\\s*[0-9]+", s.checkIntervalSec)
        s.idleCheckSec = intVal("idle_check\\s*:\\s*[0-9]+", s.idleCheckSec)
        s.cooldownSec = intVal("cooldown\\s*:\\s*[0-9]+", s.cooldownSec)
        s.killGraceSec = intVal("kill_grace\\s*:\\s*[0-9]+", s.killGraceSec)
        s.collectDiskIO = boolVal("disk_io\\s*:\\s*(true|false)", s.collectDiskIO)
        return s
    }

    func renderYAML() -> String {
        return """
        # macmon quick settings (generated from GUI)
        thresholds:
          ram_free_percent: \(ramFreePercent)
          swap_used_mb: \(swapUsedMB)
          process_ram_min_kb: \(processMinRAMKB)
          idle_cpu_percent: \(String(format: "%.2f", idleCPUPercent))

        intervals:
          check: \(checkIntervalSec)
          idle_check: \(idleCheckSec)
          cooldown: \(cooldownSec)
          kill_grace: \(killGraceSec)

        collect:
          disk_io: \(collectDiskIO ? "true" : "false")
        """
    }
}
