import Foundation

// MARK: - Data Model

struct ProcessEntry: Codable {
    let pid: Int
    let name: String
    let ramMB: Double
    let cpuPct: Double
    let uptime: String
    let uptimeSeconds: Int
    let cwd: String
    let tty: String
    let idle: Bool
    let detail: String
    let group: String
    let isSystem: Bool
    let state: String
    let diskReadMB: Double
    let diskWriteMB: Double

    var selected: Bool = false

    enum CodingKeys: String, CodingKey {
        case pid, name, ramMB, cpuPct, uptime, uptimeSeconds, cwd, tty, idle, detail, group, isSystem, state, diskReadMB, diskWriteMB
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        pid = try c.decode(Int.self, forKey: .pid)
        name = try c.decode(String.self, forKey: .name)
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

    init(pid: Int, name: String, ramMB: Double, cpuPct: Double, uptime: String = "", uptimeSeconds: Int = 0, cwd: String = "", tty: String = "", idle: Bool = false, detail: String = "", group: String = "", isSystem: Bool = false, state: String = "", diskReadMB: Double = 0, diskWriteMB: Double = 0) {
        self.pid = pid
        self.name = name
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

    var totalRAM: Double { return 0 }  // computed on demand from view model
    var count: Int { return entries.count }
}

// MARK: - View Model

class ProcessViewModel {
    var allProcesses: [ProcessEntry] = []
    var filteredIndices: [Int] = []
    var groups: [String: ProcessGroup] = [:]
    var groupOrder: [String] = []
    var groupingEnabled: Bool = false

    // Display rows: either flat indices or group headers + entries
    var displayRows: [DisplayRow] = []

    var sortColumn: SortColumn = .ramMB
    var sortOrder: SortOrder = .descending
    var searchText: String = ""

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

        if !groupingEnabled {
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
