import Cocoa
import Foundation

// MARK: - Memory Pressure Gauge

class MemoryPressureGauge: NSView {
    var freePercent: Int = 100 {
        didSet { needsDisplay = true }
    }
    var physMemGB: Double = 0
    var swapUsedMB: Int = 0

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)

        let bounds = self.bounds
        let barHeight: CGFloat = 14
        let barY = (bounds.height - barHeight) / 2

        // Background
        let bgRect = NSRect(x: 0, y: barY, width: bounds.width, height: barHeight)
        let bgPath = NSBezierPath(roundedRect: bgRect, xRadius: 7, yRadius: 7)
        NSColor.separatorColor.setFill()
        bgPath.fill()

        // Fill based on usage (100 - free = used)
        let usedPercent = max(0, min(100, 100 - freePercent))
        let fillWidth = bounds.width * CGFloat(usedPercent) / 100.0
        let fillRect = NSRect(x: 0, y: barY, width: fillWidth, height: barHeight)
        let fillPath = NSBezierPath(roundedRect: fillRect, xRadius: 7, yRadius: 7)

        let fillColor: NSColor
        if usedPercent >= 80 {
            fillColor = NSColor.systemRed
        } else if usedPercent >= 60 {
            fillColor = NSColor.systemYellow
        } else {
            fillColor = NSColor.systemGreen
        }
        fillColor.setFill()
        fillPath.fill()

        // Label
        let label = String(format: "RAM: %d%% used of %.0fGB", usedPercent, physMemGB)
        let attrs: [NSAttributedString.Key: Any] = [
            .font: NSFont.monospacedSystemFont(ofSize: 10, weight: .medium),
            .foregroundColor: NSColor.labelColor
        ]
        let str = NSAttributedString(string: label, attributes: attrs)
        let strSize = str.size()
        let strPoint = NSPoint(x: (bounds.width - strSize.width) / 2, y: barY + (barHeight - strSize.height) / 2)
        str.draw(at: strPoint)
    }
}

// MARK: - System Summary View

class SystemSummaryView: NSView {
    let memGauge = MemoryPressureGauge()
    let statsLabel = NSTextField(labelWithString: "")

    override init(frame: NSRect) {
        super.init(frame: frame)
        setup()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        setup()
    }

    private func setup() {
        memGauge.translatesAutoresizingMaskIntoConstraints = false
        statsLabel.translatesAutoresizingMaskIntoConstraints = false
        statsLabel.font = NSFont.monospacedSystemFont(ofSize: 11, weight: .regular)
        statsLabel.textColor = .secondaryLabelColor
        statsLabel.lineBreakMode = .byTruncatingTail

        addSubview(memGauge)
        addSubview(statsLabel)

        NSLayoutConstraint.activate([
            memGauge.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 8),
            memGauge.centerYAnchor.constraint(equalTo: centerYAnchor),
            memGauge.widthAnchor.constraint(equalToConstant: 280),
            memGauge.heightAnchor.constraint(equalToConstant: 20),

            statsLabel.leadingAnchor.constraint(equalTo: memGauge.trailingAnchor, constant: 16),
            statsLabel.trailingAnchor.constraint(lessThanOrEqualTo: trailingAnchor, constant: -8),
            statsLabel.centerYAnchor.constraint(equalTo: centerYAnchor),
        ])
    }

    func update(health: SystemHealth) {
        memGauge.freePercent = health.freePercent
        memGauge.physMemGB = health.physMemGB
        memGauge.swapUsedMB = health.swapUsedMB

        let stats = String(format: "Swap: %dMB  |  Procs: %d  |  Monitored: %d  |  Idle: %d",
                           health.swapUsedMB, health.totalProcesses, health.monitoredCount, health.idleCount)
        statsLabel.stringValue = stats
    }
}

// MARK: - Table Cell Identifiers

private let CellCheckbox = NSUserInterfaceItemIdentifier("CheckboxCell")
private let CellText = NSUserInterfaceItemIdentifier("TextCell")
private let CellIdle = NSUserInterfaceItemIdentifier("IdleCell")
private let CellGroupHeader = NSUserInterfaceItemIdentifier("GroupHeaderCell")

// MARK: - Column Identifiers

private let ColCheck = NSUserInterfaceItemIdentifier("check")
private let ColName = NSUserInterfaceItemIdentifier("name")
private let ColRAM = NSUserInterfaceItemIdentifier("ramMB")
private let ColCPU = NSUserInterfaceItemIdentifier("cpuPct")
private let ColUptime = NSUserInterfaceItemIdentifier("uptime")
private let ColPID = NSUserInterfaceItemIdentifier("pid")
private let ColTTY = NSUserInterfaceItemIdentifier("tty")
private let ColCWD = NSUserInterfaceItemIdentifier("cwd")
private let ColDetail = NSUserInterfaceItemIdentifier("detail")
private let ColIdle = NSUserInterfaceItemIdentifier("idle")
private let ColGroup = NSUserInterfaceItemIdentifier("group")
private let ColState = NSUserInterfaceItemIdentifier("state")
private let ColDiskR = NSUserInterfaceItemIdentifier("diskReadMB")
private let ColDiskW = NSUserInterfaceItemIdentifier("diskWriteMB")

// MARK: - Main Window Controller

class ProcessPickerController: NSObject, NSTableViewDataSource, NSTableViewDelegate, NSSearchFieldDelegate {
    let viewModel = ProcessViewModel()
    var systemHealth: SystemHealth?

    weak var tableView: NSTableView!
    var window: NSWindow!
    var statusLabel: NSTextField!
    var summaryView: SystemSummaryView!
    var searchField: NSSearchField!

    var exitCode: Int32 = 2  // default: cancelled

    func setupWindow() {
        // Window
        let contentRect = NSRect(x: 0, y: 0, width: 1100, height: 600)
        window = NSWindow(contentRect: contentRect,
                          styleMask: [.titled, .closable, .resizable, .miniaturizable],
                          backing: .buffered,
                          defer: false)
        window.title = "macmon - Process Picker"
        window.minSize = NSSize(width: 700, height: 350)
        window.setFrameAutosaveName("macmon.ProcessPicker")
        window.isReleasedWhenClosed = false
        window.center()

        let contentView = NSView(frame: contentRect)
        window.contentView = contentView

        // System summary bar (top)
        summaryView = SystemSummaryView(frame: .zero)
        summaryView.translatesAutoresizingMaskIntoConstraints = false
        contentView.addSubview(summaryView)

        // Search field
        searchField = NSSearchField(frame: .zero)
        searchField.translatesAutoresizingMaskIntoConstraints = false
        searchField.placeholderString = "Filter by name, PID, detail, directory..."
        searchField.delegate = self
        searchField.sendsSearchStringImmediately = true
        searchField.sendsWholeSearchString = false
        contentView.addSubview(searchField)

        // Table view
        let scrollView = NSScrollView(frame: .zero)
        scrollView.translatesAutoresizingMaskIntoConstraints = false
        scrollView.hasVerticalScroller = true
        scrollView.hasHorizontalScroller = true
        scrollView.autohidesScrollers = true

        let table = NSTableView(frame: .zero)
        table.style = .plain
        table.usesAlternatingRowBackgroundColors = true
        table.allowsMultipleSelection = false
        table.rowHeight = 28
        table.intercellSpacing = NSSize(width: 8, height: 0)
        table.dataSource = self
        table.delegate = self
        self.tableView = table

        // Define columns
        let columns: [(NSUserInterfaceItemIdentifier, String, CGFloat, CGFloat)] = [
            (ColCheck, "", 30, 30),
            (ColName, "Name", 160, 80),
            (ColRAM, "RAM (MB)", 80, 60),
            (ColCPU, "CPU %", 65, 50),
            (ColUptime, "Uptime", 85, 60),
            (ColPID, "PID", 65, 45),
            (ColDetail, "Detail", 200, 80),
            (ColCWD, "Directory", 200, 80),
            (ColIdle, "Idle", 40, 35),
            (ColGroup, "Group", 100, 60),
            (ColDiskR, "Disk R (MB)", 85, 60),
            (ColDiskW, "Disk W (MB)", 85, 60),
            (ColState, "State", 50, 40),
            (ColTTY, "TTY", 70, 50),
        ]

        for (id, title, width, minWidth) in columns {
            let col = NSTableColumn(identifier: id)
            col.title = title
            col.width = width
            col.minWidth = minWidth
            if id != ColCheck {
                col.sortDescriptorPrototype = NSSortDescriptor(key: id.rawValue, ascending: true)
            }
            if id == ColCheck {
                col.headerCell.title = ""
                col.isEditable = false
                col.resizingMask = []
            } else {
                col.resizingMask = [.autoresizingMask, .userResizingMask]
            }
            table.addTableColumn(col)
        }

        scrollView.documentView = table
        contentView.addSubview(scrollView)

        // Status bar
        statusLabel = NSTextField(labelWithString: "")
        statusLabel.translatesAutoresizingMaskIntoConstraints = false
        statusLabel.font = NSFont.monospacedSystemFont(ofSize: 11, weight: .regular)
        statusLabel.textColor = .secondaryLabelColor
        contentView.addSubview(statusLabel)

        // Buttons
        let btnSelectAll = NSButton(title: "Select All", target: self, action: #selector(selectAll))
        let btnSelectNone = NSButton(title: "Select None", target: self, action: #selector(selectNone))
        let btnSelectIdle = NSButton(title: "Select Idle", target: self, action: #selector(selectIdle))
        let btnToggleGroups = NSButton(title: "Groups", target: self, action: #selector(toggleGrouping))
        let btnCancel = NSButton(title: "Cancel", target: self, action: #selector(cancelAction))
        let btnClose = NSButton(title: "Close Selected", target: self, action: #selector(closeSelected))

        btnClose.bezelColor = NSColor.systemRed
        btnCancel.keyEquivalent = "\u{1b}"  // Escape
        btnClose.keyEquivalent = "\r"       // Enter

        let buttons = [btnSelectAll, btnSelectNone, btnSelectIdle, btnToggleGroups, btnCancel, btnClose]
        for btn in buttons {
            btn.translatesAutoresizingMaskIntoConstraints = false
            btn.bezelStyle = .rounded
            contentView.addSubview(btn)
        }

        // Layout
        NSLayoutConstraint.activate([
            // Summary bar
            summaryView.topAnchor.constraint(equalTo: contentView.topAnchor, constant: 8),
            summaryView.leadingAnchor.constraint(equalTo: contentView.leadingAnchor),
            summaryView.trailingAnchor.constraint(equalTo: contentView.trailingAnchor),
            summaryView.heightAnchor.constraint(equalToConstant: 30),

            // Search field
            searchField.topAnchor.constraint(equalTo: summaryView.bottomAnchor, constant: 8),
            searchField.leadingAnchor.constraint(equalTo: contentView.leadingAnchor, constant: 8),
            searchField.trailingAnchor.constraint(equalTo: contentView.trailingAnchor, constant: -8),

            // Table
            scrollView.topAnchor.constraint(equalTo: searchField.bottomAnchor, constant: 8),
            scrollView.leadingAnchor.constraint(equalTo: contentView.leadingAnchor),
            scrollView.trailingAnchor.constraint(equalTo: contentView.trailingAnchor),
            scrollView.bottomAnchor.constraint(equalTo: statusLabel.topAnchor, constant: -4),

            // Status
            statusLabel.leadingAnchor.constraint(equalTo: contentView.leadingAnchor, constant: 8),
            statusLabel.bottomAnchor.constraint(equalTo: btnClose.topAnchor, constant: -8),

            // Buttons (bottom row)
            btnSelectAll.leadingAnchor.constraint(equalTo: contentView.leadingAnchor, constant: 8),
            btnSelectAll.bottomAnchor.constraint(equalTo: contentView.bottomAnchor, constant: -8),

            btnSelectNone.leadingAnchor.constraint(equalTo: btnSelectAll.trailingAnchor, constant: 4),
            btnSelectNone.bottomAnchor.constraint(equalTo: contentView.bottomAnchor, constant: -8),

            btnSelectIdle.leadingAnchor.constraint(equalTo: btnSelectNone.trailingAnchor, constant: 4),
            btnSelectIdle.bottomAnchor.constraint(equalTo: contentView.bottomAnchor, constant: -8),

            btnToggleGroups.leadingAnchor.constraint(equalTo: btnSelectIdle.trailingAnchor, constant: 4),
            btnToggleGroups.bottomAnchor.constraint(equalTo: contentView.bottomAnchor, constant: -8),

            btnClose.trailingAnchor.constraint(equalTo: contentView.trailingAnchor, constant: -8),
            btnClose.bottomAnchor.constraint(equalTo: contentView.bottomAnchor, constant: -8),

            btnCancel.trailingAnchor.constraint(equalTo: btnClose.leadingAnchor, constant: -4),
            btnCancel.bottomAnchor.constraint(equalTo: contentView.bottomAnchor, constant: -8),
        ])
    }

    // MARK: - Data Loading

    func loadData(from file: String) -> Bool {
        guard let data = FileManager.default.contents(atPath: file) else {
            fputs("Error: Cannot read file \(file)\n", stderr)
            return false
        }

        let decoder = JSONDecoder()
        do {
            let processData = try decoder.decode(ProcessData.self, from: data)
            viewModel.load(from: processData)
            systemHealth = processData.system
            if let health = systemHealth {
                summaryView.update(health: health)
            }
            updateStatus()
            return true
        } catch {
            fputs("Error: Failed to parse JSON: \(error)\n", stderr)
            return false
        }
    }

    // MARK: - Table Data Source

    func numberOfRows(in tableView: NSTableView) -> Int {
        return viewModel.displayRows.count
    }

    // MARK: - Table Delegate

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        guard row < viewModel.displayRows.count else { return nil }
        guard let column = tableColumn else { return nil }

        let displayRow = viewModel.displayRows[row]

        switch displayRow {
        case .groupHeader(let name, let count, let totalRAM, let collapsed):
            if column.identifier == ColCheck { return nil }
            if column.identifier == ColName {
                let cell = recycleOrCreateTextCell(tableView, id: CellGroupHeader)
                let arrow = collapsed ? "▶" : "▼"
                cell.stringValue = "\(arrow) \(name) (\(count) processes, \(String(format: "%.0f", totalRAM)) MB)"
                cell.font = NSFont.systemFont(ofSize: 12, weight: .semibold)
                cell.textColor = .labelColor
                return cell
            }
            return nil

        case .process(let idx):
            guard idx < viewModel.allProcesses.count else { return nil }
            let proc = viewModel.allProcesses[idx]

            if column.identifier == ColCheck {
                return checkboxCell(tableView, proc: proc, index: idx)
            }

            if column.identifier == ColIdle {
                return idleCell(tableView, idle: proc.idle)
            }

            let cell = recycleOrCreateTextCell(tableView, id: CellText)
            configureTextCell(cell, column: column.identifier, proc: proc)
            return cell
        }
    }

    func tableView(_ tableView: NSTableView, heightOfRow row: Int) -> CGFloat {
        guard row < viewModel.displayRows.count else { return 28 }
        switch viewModel.displayRows[row] {
        case .groupHeader: return 32
        case .process: return 28
        }
    }

    func tableView(_ tableView: NSTableView, isGroupRow row: Int) -> Bool {
        guard row < viewModel.displayRows.count else { return false }
        if case .groupHeader = viewModel.displayRows[row] { return true }
        return false
    }

    func tableView(_ tableView: NSTableView, shouldSelectRow row: Int) -> Bool {
        guard row < viewModel.displayRows.count else { return false }
        if case .groupHeader(let name, _, _, _) = viewModel.displayRows[row] {
            viewModel.toggleGroup(name)
            tableView.reloadData()
            return false
        }
        return true
    }

    func tableView(_ tableView: NSTableView, rowViewForRow row: Int) -> NSTableRowView? {
        guard row < viewModel.displayRows.count else { return nil }
        if case .process(let idx) = viewModel.displayRows[row] {
            if viewModel.allProcesses[idx].selected {
                let rv = NSTableRowView()
                rv.isEmphasized = true
                return rv
            }
        }
        return nil
    }

    func tableView(_ tableView: NSTableView, sortDescriptorsDidChange oldDescriptors: [NSSortDescriptor]) {
        guard let descriptor = tableView.sortDescriptors.first,
              let key = descriptor.key,
              let col = SortColumn(rawValue: key) else { return }

        if viewModel.sortColumn == col {
            viewModel.sortOrder.toggle()
        } else {
            viewModel.sortColumn = col
            viewModel.sortOrder = descriptor.ascending ? .ascending : .descending
        }
        viewModel.applyFilterAndSort()
        tableView.reloadData()
    }

    // MARK: - Cell Creation with Recycling

    private func recycleOrCreateTextCell(_ tableView: NSTableView, id: NSUserInterfaceItemIdentifier) -> NSTextField {
        if let existing = tableView.makeView(withIdentifier: id, owner: self) as? NSTextField {
            return existing
        }
        let cell = NSTextField(labelWithString: "")
        cell.identifier = id
        cell.font = NSFont.monospacedSystemFont(ofSize: 11, weight: .regular)
        cell.lineBreakMode = .byTruncatingTail
        cell.isEditable = false
        cell.isBordered = false
        cell.drawsBackground = false
        cell.translatesAutoresizingMaskIntoConstraints = false
        return cell
    }

    private func checkboxCell(_ tableView: NSTableView, proc: ProcessEntry, index: Int) -> NSView {
        if let existing = tableView.makeView(withIdentifier: CellCheckbox, owner: self),
           let cb = existing.subviews.first as? NSButton {
            cb.state = proc.selected ? .on : .off
            cb.tag = index
            return existing
        }
        let wrapper = NSView()
        wrapper.identifier = CellCheckbox
        let cb = NSButton(checkboxWithTitle: "", target: self, action: #selector(checkboxToggled(_:)))
        cb.state = proc.selected ? .on : .off
        cb.tag = index
        cb.translatesAutoresizingMaskIntoConstraints = false
        wrapper.addSubview(cb)
        NSLayoutConstraint.activate([
            cb.centerXAnchor.constraint(equalTo: wrapper.centerXAnchor),
            cb.centerYAnchor.constraint(equalTo: wrapper.centerYAnchor),
        ])
        return wrapper
    }

    private func idleCell(_ tableView: NSTableView, idle: Bool) -> NSView {
        if let existing = tableView.makeView(withIdentifier: CellIdle, owner: self) as? NSTextField {
            existing.stringValue = idle ? "💤" : ""
            return existing
        }
        let cell = NSTextField(labelWithString: idle ? "💤" : "")
        cell.identifier = CellIdle
        cell.alignment = .center
        cell.font = NSFont.systemFont(ofSize: 12)
        cell.isEditable = false
        cell.isBordered = false
        cell.drawsBackground = false
        return cell
    }

    private func configureTextCell(_ cell: NSTextField, column: NSUserInterfaceItemIdentifier, proc: ProcessEntry) {
        cell.textColor = .labelColor
        cell.font = NSFont.monospacedSystemFont(ofSize: 11, weight: .regular)

        switch column {
        case ColName:
            cell.stringValue = proc.name
            cell.font = NSFont.systemFont(ofSize: 12, weight: .medium)
            cell.textColor = .labelColor
        case ColRAM:
            cell.stringValue = String(format: "%.1f", proc.ramMB)
            cell.alignment = .right
            if proc.ramMB > 2048 {
                cell.textColor = .systemRed
            } else if proc.ramMB > 512 {
                cell.textColor = .systemOrange
            }
        case ColCPU:
            cell.stringValue = String(format: "%.1f", proc.cpuPct)
            cell.alignment = .right
            if proc.cpuPct > 80 {
                cell.textColor = .systemRed
            } else if proc.cpuPct > 30 {
                cell.textColor = .systemOrange
            }
        case ColUptime:
            cell.stringValue = proc.uptime
        case ColPID:
            cell.stringValue = String(proc.pid)
            cell.alignment = .right
        case ColTTY:
            cell.stringValue = proc.tty == "??" ? "-" : proc.tty
        case ColCWD:
            if proc.cwd.isEmpty {
                cell.stringValue = "-"
                cell.textColor = .tertiaryLabelColor
            } else {
                // Show last 2 path components
                let components = proc.cwd.split(separator: "/")
                if components.count > 2 {
                    cell.stringValue = "…/" + components.suffix(2).joined(separator: "/")
                } else {
                    cell.stringValue = proc.cwd
                }
            }
        case ColDetail:
            cell.stringValue = proc.detail.isEmpty ? "-" : proc.detail
            if proc.detail.isEmpty { cell.textColor = .tertiaryLabelColor }
        case ColGroup:
            cell.stringValue = proc.group.isEmpty ? "-" : proc.group
            if proc.group.isEmpty { cell.textColor = .tertiaryLabelColor }
        case ColDiskR:
            cell.stringValue = proc.diskReadMB > 0 ? String(format: "%.1f", proc.diskReadMB) : "-"
            cell.alignment = .right
            if proc.diskReadMB > 10000 { cell.textColor = .systemRed }
            else if proc.diskReadMB > 1000 { cell.textColor = .systemOrange }
        case ColDiskW:
            cell.stringValue = proc.diskWriteMB > 0 ? String(format: "%.1f", proc.diskWriteMB) : "-"
            cell.alignment = .right
            if proc.diskWriteMB > 10000 { cell.textColor = .systemRed }
            else if proc.diskWriteMB > 1000 { cell.textColor = .systemOrange }
        case ColState:
            cell.stringValue = proc.state
        default:
            cell.stringValue = ""
        }
    }

    // MARK: - Actions

    @objc func checkboxToggled(_ sender: NSButton) {
        let index = sender.tag
        guard index >= 0 && index < viewModel.allProcesses.count else { return }
        viewModel.allProcesses[index].selected = (sender.state == .on)
        // Reload only the affected row
        if let row = viewModel.displayRows.firstIndex(where: {
            if case .process(let idx) = $0 { return idx == index }
            return false
        }) {
            tableView.reloadData(forRowIndexes: IndexSet(integer: row),
                                 columnIndexes: IndexSet(integersIn: 0..<tableView.numberOfColumns))
        }
        updateStatus()
    }

    @objc func selectAll(_ sender: Any?) {
        for i in viewModel.filteredIndices {
            if !viewModel.allProcesses[i].isSystem {
                viewModel.allProcesses[i].selected = true
            }
        }
        tableView.reloadData()
        updateStatus()
    }

    @objc func selectNone(_ sender: Any?) {
        for i in 0..<viewModel.allProcesses.count {
            viewModel.allProcesses[i].selected = false
        }
        tableView.reloadData()
        updateStatus()
    }

    @objc func selectIdle(_ sender: Any?) {
        for i in viewModel.filteredIndices {
            if viewModel.allProcesses[i].idle && !viewModel.allProcesses[i].isSystem {
                viewModel.allProcesses[i].selected = true
            }
        }
        tableView.reloadData()
        updateStatus()
    }

    @objc func toggleGrouping(_ sender: Any?) {
        viewModel.groupingEnabled.toggle()
        viewModel.applyFilterAndSort()
        tableView.reloadData()
    }

    @objc func cancelAction(_ sender: Any?) {
        exitCode = 2
        NSApp.terminate(nil)
    }

    @objc func closeSelected(_ sender: Any?) {
        let pids = viewModel.selectedPIDs
        if pids.isEmpty {
            exitCode = 0
            NSApp.terminate(nil)
            return
        }
        // Output selected PIDs to stdout
        for pid in pids {
            print(pid)
        }
        exitCode = 0
        NSApp.terminate(nil)
    }

    // MARK: - Search

    func controlTextDidChange(_ obj: Notification) {
        if let field = obj.object as? NSSearchField, field === searchField {
            viewModel.searchText = field.stringValue
            viewModel.applyFilterAndSort()
            tableView.reloadData()
            updateStatus()
        }
    }

    // MARK: - Status

    func updateStatus() {
        let total = viewModel.filteredIndices.count
        let selected = viewModel.selectedCount
        let ramTotal = viewModel.selectedRAM
        let idleCount = viewModel.allProcesses.filter { $0.idle }.count

        var text = "\(total) processes"
        if !viewModel.searchText.isEmpty {
            text += " (filtered from \(viewModel.allProcesses.count))"
        }
        text += "  |  \(idleCount) idle"
        if selected > 0 {
            text += String(format: "  |  %d selected (%.0f MB)", selected, ramTotal)
        }
        statusLabel.stringValue = text
    }
}

// MARK: - App Delegate

class AppDelegate: NSObject, NSApplicationDelegate {
    let controller = ProcessPickerController()
    var inputFile: String?

    func applicationDidFinishLaunching(_ notification: Notification) {
        controller.setupWindow()

        guard let file = inputFile else {
            fputs("Error: No input file specified. Use --file <path>\n", stderr)
            exit(1)
        }

        if !controller.loadData(from: file) {
            exit(1)
        }

        controller.tableView.reloadData()
        controller.window.makeKeyAndOrderFront(nil)

        // Set up keyboard shortcuts
        NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard let self = self else { return event }
            if event.modifierFlags.contains(.command) {
                switch event.charactersIgnoringModifiers {
                case "a":
                    self.controller.selectAll(nil)
                    return nil
                case "f":
                    self.controller.window.makeFirstResponder(self.controller.searchField)
                    return nil
                case "g":
                    self.controller.toggleGrouping(nil)
                    return nil
                default:
                    break
                }
            }
            if event.keyCode == 51 { // Delete key
                self.controller.closeSelected(nil)
                return nil
            }
            return event
        }
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        return true
    }

    func applicationWillTerminate(_ notification: Notification) {
        exit(controller.exitCode)
    }
}

// MARK: - Main

@main
struct ProcessPickerApp {
    static func main() {
        let args = CommandLine.arguments
        var inputFile: String?

        var i = 1
        while i < args.count {
            if args[i] == "--file" && i + 1 < args.count {
                inputFile = args[i + 1]
                i += 2
            } else {
                i += 1
            }
        }

        let app = NSApplication.shared
        app.setActivationPolicy(.regular)
        let delegate = AppDelegate()
        delegate.inputFile = inputFile
        app.delegate = delegate
        app.run()
    }
}
