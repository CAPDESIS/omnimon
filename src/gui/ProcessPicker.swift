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
        let label = LF("picker.memory.label", usedPercent, physMemGB)
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

class MiniBarChartView: NSView {
    var values: [Double] = [0, 0, 0] { didSet { needsDisplay = true } }
    var labels: [String] = ["RAM", "Swap", "Proc"]

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        guard values.count == labels.count else { return }

        let barWidth: CGFloat = 48
        let gap: CGFloat = 8
        let maxHeight: CGFloat = 28
        let baseY: CGFloat = 14

        for i in 0..<values.count {
            let x = CGFloat(i) * (barWidth + gap)
            let v = max(0, min(1, values[i]))
            let h = maxHeight * CGFloat(v)

            let bgRect = NSRect(x: x, y: baseY, width: barWidth, height: maxHeight)
            NSColor.separatorColor.withAlphaComponent(0.35).setFill()
            NSBezierPath(roundedRect: bgRect, xRadius: 3, yRadius: 3).fill()

            let fgRect = NSRect(x: x, y: baseY, width: barWidth, height: h)
            let color: NSColor = v > 0.8 ? .systemRed : (v > 0.6 ? .systemOrange : .systemGreen)
            color.setFill()
            NSBezierPath(roundedRect: fgRect, xRadius: 3, yRadius: 3).fill()

            let text = NSAttributedString(string: labels[i], attributes: [
                .font: NSFont.systemFont(ofSize: 9, weight: .medium),
                .foregroundColor: NSColor.secondaryLabelColor,
            ])
            text.draw(at: NSPoint(x: x + 10, y: 1))
        }
    }
}

// MARK: - System Summary View

class SystemSummaryView: NSView {
    let memGauge = MemoryPressureGauge()
    let statsLabel = NSTextField(labelWithString: "")
    let chartView = MiniBarChartView()

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
        chartView.translatesAutoresizingMaskIntoConstraints = false

        addSubview(memGauge)
        addSubview(statsLabel)
        addSubview(chartView)

        NSLayoutConstraint.activate([
            memGauge.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 8),
            memGauge.centerYAnchor.constraint(equalTo: centerYAnchor),
            memGauge.widthAnchor.constraint(equalToConstant: 280),
            memGauge.heightAnchor.constraint(equalToConstant: 20),

            statsLabel.leadingAnchor.constraint(equalTo: memGauge.trailingAnchor, constant: 16),
            statsLabel.trailingAnchor.constraint(lessThanOrEqualTo: chartView.leadingAnchor, constant: -12),
            statsLabel.centerYAnchor.constraint(equalTo: centerYAnchor),

            chartView.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -8),
            chartView.centerYAnchor.constraint(equalTo: centerYAnchor),
            chartView.widthAnchor.constraint(equalToConstant: 170),
            chartView.heightAnchor.constraint(equalToConstant: 46),
        ])
    }

    func update(health: SystemHealth) {
        memGauge.freePercent = health.freePercent
        memGauge.physMemGB = health.physMemGB
        memGauge.swapUsedMB = health.swapUsedMB

        let stats = LF("picker.stats.label",
                       health.swapUsedMB, health.totalProcesses, health.monitoredCount, health.idleCount)
        statsLabel.stringValue = stats

        let ramUsedRatio = Double(max(0, min(100, 100 - health.freePercent))) / 100.0
        let swapRatio = min(1.0, Double(max(0, health.swapUsedMB)) / 3072.0)
        let procRatio = min(1.0, Double(max(0, health.totalProcesses)) / 1000.0)
        chartView.values = [ramUsedRatio, swapRatio, procRatio]
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
    private let dataQueue = DispatchQueue(label: "com.macmon.processpicker.data", qos: .userInitiated)

    weak var tableView: NSTableView!
    var window: NSWindow!
    var statusLabel: NSTextField!
    var inspectorLabel: NSTextField!
    var helperLabel: NSTextField!
    var profileHintLabel: NSTextField!
    var summaryView: SystemSummaryView!
    var searchField: NSSearchField!
    var hideSystemCheckbox: NSButton!
    var idleOnlyCheckbox: NSButton!
    var closeButton: NSButton!
    var cancelButton: NSButton!
    var commandPopup: NSPopUpButton!
    var profilePopup: NSPopUpButton!
    var preferencesWindowController: PreferencesWindowController?
    var configWindow: NSWindow?
    var configTextView: NSTextView?
    var configPath: String = ""
    var configFields: [String: NSTextField] = [:]
    var configDiskIOCheckbox: NSButton?
    private let aiBlockedNames = AIService.immutableProtectedProcessNames
    private var smartOptimizeButton: NSButton?
    private var aiSpinner: NSProgressIndicator?
    private var isAnalyzingAI = false

    var exitCode: Int32 = 2  // default: cancelled

    func setupWindow() {
        // Window
        let contentRect = NSRect(x: 0, y: 0, width: 1280, height: 720)
        window = NSWindow(contentRect: contentRect,
                          styleMask: [.titled, .closable, .resizable, .miniaturizable],
                          backing: .buffered,
                          defer: false)
        window.title = L("picker.window.title")
        window.minSize = NSSize(width: 960, height: 460)
        window.setFrameAutosaveName("macmon.ProcessPicker")
        window.isReleasedWhenClosed = false
        window.center()

        let contentView = NSView(frame: contentRect)
        window.contentView = contentView

        // System summary bar (top)
        summaryView = SystemSummaryView(frame: .zero)
        summaryView.translatesAutoresizingMaskIntoConstraints = false
        contentView.addSubview(summaryView)
        let chartClick = NSClickGestureRecognizer(target: self, action: #selector(showSummaryInsights(_:)))
        summaryView.chartView.addGestureRecognizer(chartClick)
        summaryView.chartView.setAccessibilityRole(.button)
        summaryView.chartView.setAccessibilityLabel(L("picker.metrics.title"))
        summaryView.chartView.setAccessibilityHelp(L("picker.metrics.help"))

        // Search field
        searchField = NSSearchField(frame: .zero)
        searchField.translatesAutoresizingMaskIntoConstraints = false
        searchField.placeholderString = L("picker.search.placeholder")
        searchField.delegate = self
        searchField.sendsSearchStringImmediately = true
        searchField.sendsWholeSearchString = false
        searchField.setAccessibilityLabel(L("picker.a11y.search"))
        contentView.addSubview(searchField)

        profilePopup = NSPopUpButton(frame: .zero, pullsDown: false)
        profilePopup.translatesAutoresizingMaskIntoConstraints = false
        profilePopup.target = self
        profilePopup.action = #selector(profileChanged(_:))
        profilePopup.toolTip = L("picker.profile.tooltip")
        contentView.addSubview(profilePopup)

        profileHintLabel = NSTextField(labelWithString: "")
        profileHintLabel.translatesAutoresizingMaskIntoConstraints = false
        profileHintLabel.font = NSFont.systemFont(ofSize: 11, weight: .regular)
        profileHintLabel.textColor = .secondaryLabelColor
        profileHintLabel.lineBreakMode = .byTruncatingTail
        profileHintLabel.setAccessibilityLabel(L("picker.profile.hint"))
        contentView.addSubview(profileHintLabel)

        hideSystemCheckbox = NSButton(checkboxWithTitle: L("picker.filter.hide_system"), target: self, action: #selector(filterToggled(_:)))
        hideSystemCheckbox.translatesAutoresizingMaskIntoConstraints = false
        hideSystemCheckbox.state = .off
        hideSystemCheckbox.toolTip = L("picker.filter.hide_system.help")
        contentView.addSubview(hideSystemCheckbox)

        idleOnlyCheckbox = NSButton(checkboxWithTitle: L("picker.filter.only_idle"), target: self, action: #selector(filterToggled(_:)))
        idleOnlyCheckbox.translatesAutoresizingMaskIntoConstraints = false
        idleOnlyCheckbox.state = .off
        idleOnlyCheckbox.toolTip = L("picker.filter.only_idle.help")
        contentView.addSubview(idleOnlyCheckbox)
        reloadProfiles()

        // Table view
        let scrollView = NSScrollView(frame: .zero)
        scrollView.translatesAutoresizingMaskIntoConstraints = false
        scrollView.hasVerticalScroller = true
        scrollView.hasHorizontalScroller = true
        scrollView.autohidesScrollers = true

        let table = NSTableView(frame: .zero)
        table.style = .plain
        table.usesAlternatingRowBackgroundColors = true
        table.columnAutoresizingStyle = .lastColumnOnlyAutoresizingStyle
        table.allowsColumnReordering = true
        table.autosaveName = "macmon.ProcessPicker.Columns"
        table.autosaveTableColumns = true
        table.allowsMultipleSelection = false
        table.rowHeight = 28
        table.intercellSpacing = NSSize(width: 8, height: 0)
        table.dataSource = self
        table.delegate = self
        table.setAccessibilityRole(.table)
        table.setAccessibilityLabel(L("picker.a11y.table"))
        self.tableView = table

        // Define columns
        let columns: [(NSUserInterfaceItemIdentifier, String, CGFloat, CGFloat)] = [
            (ColCheck, "", 30, 30),
            (ColGroup, L("picker.column.group"), 160, 110),
            (ColName, L("picker.column.name"), 280, 180),
            (ColRAM, L("picker.column.ram"), 80, 60),
            (ColCPU, L("picker.column.cpu"), 65, 50),
            (ColUptime, L("picker.column.uptime"), 85, 60),
            (ColPID, L("picker.column.pid"), 65, 45),
            (ColDetail, L("picker.column.detail"), 200, 80),
            (ColDiskR, L("picker.column.disk_r"), 95, 70),
            (ColDiskW, L("picker.column.disk_w"), 95, 70),
            (ColIdle, L("picker.column.idle"), 65, 55),
            (ColCWD, L("picker.column.directory"), 240, 140),
            (ColState, L("picker.column.state"), 50, 40),
            (ColTTY, L("picker.column.tty"), 70, 50),
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
        statusLabel.lineBreakMode = .byTruncatingTail
        statusLabel.maximumNumberOfLines = 1
        contentView.addSubview(statusLabel)

        inspectorLabel = NSTextField(wrappingLabelWithString: "")
        inspectorLabel.translatesAutoresizingMaskIntoConstraints = false
        inspectorLabel.font = NSFont.systemFont(ofSize: 11, weight: .regular)
        inspectorLabel.textColor = .secondaryLabelColor
        inspectorLabel.maximumNumberOfLines = 2
        inspectorLabel.lineBreakMode = .byTruncatingTail
        contentView.addSubview(inspectorLabel)

        helperLabel = NSTextField(labelWithString: L("picker.smart_optimize.help_inline"))
        helperLabel.translatesAutoresizingMaskIntoConstraints = false
        helperLabel.font = NSFont.systemFont(ofSize: 11, weight: .regular)
        helperLabel.textColor = .secondaryLabelColor
        helperLabel.lineBreakMode = .byTruncatingTail
        helperLabel.maximumNumberOfLines = 1
        contentView.addSubview(helperLabel)

        // Buttons
        let btnSelectAll = NSButton(title: L("picker.button.select_all"), target: self, action: #selector(selectAll))
        let btnSelectNone = NSButton(title: L("picker.button.select_none"), target: self, action: #selector(selectNone))
        let btnSelectIdle = NSButton(title: L("picker.button.select_idle"), target: self, action: #selector(selectIdle))
        let btnSelectTopRAM = NSButton(title: L("picker.button.select_top_ram"), target: self, action: #selector(selectTopRAM))
        let btnSelectTopCPU = NSButton(title: L("picker.button.select_top_cpu"), target: self, action: #selector(selectTopCPU))
        let btnToggleGroups = NSButton(title: L("picker.button.groups"), target: self, action: #selector(toggleGrouping))
        let btnSmartOptimize = NSButton(title: L("picker.button.smart_optimize"), target: self, action: #selector(smartOptimize))
        smartOptimizeButton = btnSmartOptimize
        let spinner = NSProgressIndicator()
        spinner.style = .spinning
        spinner.controlSize = .small
        spinner.isDisplayedWhenStopped = false
        spinner.translatesAutoresizingMaskIntoConstraints = false
        spinner.setAccessibilityRole(.progressIndicator)
        spinner.setAccessibilityLabel(L("picker.ai.a11y.spinner"))
        contentView.addSubview(spinner)
        aiSpinner = spinner
        let btnCancel = NSButton(title: L("picker.button.cancel"), target: self, action: #selector(cancelAction))
        let btnClose = NSButton(title: L("picker.button.close_selected"), target: self, action: #selector(closeSelected))
        commandPopup = NSPopUpButton(frame: .zero, pullsDown: true)
        commandPopup.target = self
        commandPopup.action = #selector(commandSelected(_:))
        commandPopup.addItem(withTitle: L("picker.commands.title"))
        commandPopup.lastItem?.representedObject = ""
        addCommandMenuItem(L("picker.commands.open_config"), command: "open_config")
        addCommandMenuItem(L("picker.commands.edit_config_gui"), command: "edit_config_gui")
        addCommandMenuItem(L("picker.commands.ai_settings"), command: "preferences")
        addCommandMenuItem(L("picker.commands.chrome_tabs"), command: "chrome_tabs")
        addCommandMenuItem(L("picker.commands.reset_config"), command: "reset_config")
        addCommandMenuItem(L("picker.commands.daemon_start"), command: "start")
        addCommandMenuItem(L("picker.commands.daemon_stop"), command: "stop")
        addCommandMenuItem(L("picker.commands.daemon_restart"), command: "restart")
        addCommandMenuItem(L("picker.commands.log_tail"), command: "log_tail")
        addCommandMenuItem(L("picker.commands.export_json"), command: "export_json")
        addCommandMenuItem(L("picker.commands.export_csv"), command: "export_csv")
        addCommandMenuItem(L("picker.commands.status"), command: "status")
        addCommandMenuItem(L("picker.commands.version"), command: "version")
        addCommandMenuItem(L("picker.commands.show_help"), command: "help")
        addCommandMenuItem(L("picker.commands.update"), command: "update")
        commandPopup.setAccessibilityLabel(L("picker.commands.title"))
        commandPopup.toolTip = L("picker.commands.help")
        cancelButton = btnCancel
        closeButton = btnClose

        btnClose.bezelColor = NSColor.systemRed
        btnCancel.keyEquivalent = "\u{1b}"  // Escape
        btnClose.keyEquivalent = "\r"       // Enter

        let buttons = [btnSelectAll, btnSelectNone, btnSelectIdle, btnSelectTopRAM, btnSelectTopCPU, btnToggleGroups, btnSmartOptimize, btnCancel, btnClose]
        for btn in buttons {
            btn.translatesAutoresizingMaskIntoConstraints = false
            btn.bezelStyle = .rounded
            btn.setAccessibilityRole(.button)
            btn.setAccessibilityLabel(btn.title)
            contentView.addSubview(btn)
        }
        btnSelectTopRAM.toolTip = L("picker.button.select_top_ram.help")
        btnSelectTopCPU.toolTip = L("picker.button.select_top_cpu.help")
        btnToggleGroups.toolTip = L("picker.button.groups.help")
        btnSmartOptimize.toolTip = L("picker.button.smart_optimize.help")
        btnSmartOptimize.setAccessibilityHelp(L("picker.ai.a11y.button_help"))
        commandPopup.translatesAutoresizingMaskIntoConstraints = false
        contentView.addSubview(commandPopup)

        searchField.nextKeyView = table
        table.nextKeyView = btnSelectAll
        btnSelectAll.nextKeyView = btnSelectNone
        btnSelectNone.nextKeyView = btnSelectIdle
        btnSelectIdle.nextKeyView = btnToggleGroups
        btnToggleGroups.nextKeyView = btnCancel
        btnCancel.nextKeyView = btnSmartOptimize
        btnSmartOptimize.nextKeyView = btnClose
        btnClose.nextKeyView = searchField

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
            searchField.trailingAnchor.constraint(equalTo: profilePopup.leadingAnchor, constant: -8),

            profilePopup.centerYAnchor.constraint(equalTo: searchField.centerYAnchor),
            profilePopup.trailingAnchor.constraint(equalTo: contentView.trailingAnchor, constant: -8),
            profilePopup.widthAnchor.constraint(equalToConstant: 220),

            profileHintLabel.topAnchor.constraint(equalTo: searchField.bottomAnchor, constant: 6),
            profileHintLabel.trailingAnchor.constraint(equalTo: contentView.trailingAnchor, constant: -8),
            profileHintLabel.widthAnchor.constraint(equalToConstant: 340),

            hideSystemCheckbox.topAnchor.constraint(equalTo: searchField.bottomAnchor, constant: 4),
            hideSystemCheckbox.leadingAnchor.constraint(equalTo: contentView.leadingAnchor, constant: 8),

            idleOnlyCheckbox.topAnchor.constraint(equalTo: searchField.bottomAnchor, constant: 4),
            idleOnlyCheckbox.leadingAnchor.constraint(equalTo: hideSystemCheckbox.trailingAnchor, constant: 16),

            // Table
            scrollView.topAnchor.constraint(equalTo: hideSystemCheckbox.bottomAnchor, constant: 8),
            scrollView.leadingAnchor.constraint(equalTo: contentView.leadingAnchor),
            scrollView.trailingAnchor.constraint(equalTo: contentView.trailingAnchor),
            scrollView.bottomAnchor.constraint(equalTo: inspectorLabel.topAnchor, constant: -4),

            // Status
            statusLabel.leadingAnchor.constraint(equalTo: contentView.leadingAnchor, constant: 8),
            statusLabel.bottomAnchor.constraint(equalTo: btnClose.topAnchor, constant: -8),

            inspectorLabel.leadingAnchor.constraint(equalTo: contentView.leadingAnchor, constant: 8),
            inspectorLabel.trailingAnchor.constraint(equalTo: contentView.trailingAnchor, constant: -8),
            inspectorLabel.bottomAnchor.constraint(equalTo: statusLabel.topAnchor, constant: -2),
            inspectorLabel.heightAnchor.constraint(equalToConstant: 30),

            statusLabel.trailingAnchor.constraint(equalTo: contentView.trailingAnchor, constant: -8),
            statusLabel.bottomAnchor.constraint(equalTo: helperLabel.topAnchor, constant: -2),
            statusLabel.heightAnchor.constraint(equalToConstant: 16),

            helperLabel.leadingAnchor.constraint(equalTo: contentView.leadingAnchor, constant: 8),
            helperLabel.trailingAnchor.constraint(equalTo: contentView.trailingAnchor, constant: -8),
            helperLabel.bottomAnchor.constraint(equalTo: btnClose.topAnchor, constant: -6),
            helperLabel.heightAnchor.constraint(equalToConstant: 16),

            // Buttons (bottom row)
            btnSelectAll.leadingAnchor.constraint(equalTo: contentView.leadingAnchor, constant: 8),
            btnSelectAll.bottomAnchor.constraint(equalTo: contentView.bottomAnchor, constant: -8),

            btnSelectNone.leadingAnchor.constraint(equalTo: btnSelectAll.trailingAnchor, constant: 4),
            btnSelectNone.bottomAnchor.constraint(equalTo: contentView.bottomAnchor, constant: -8),

            btnSelectIdle.leadingAnchor.constraint(equalTo: btnSelectNone.trailingAnchor, constant: 4),
            btnSelectIdle.bottomAnchor.constraint(equalTo: contentView.bottomAnchor, constant: -8),

            btnSelectTopRAM.leadingAnchor.constraint(equalTo: btnSelectIdle.trailingAnchor, constant: 4),
            btnSelectTopRAM.bottomAnchor.constraint(equalTo: contentView.bottomAnchor, constant: -8),

            btnSelectTopCPU.leadingAnchor.constraint(equalTo: btnSelectTopRAM.trailingAnchor, constant: 4),
            btnSelectTopCPU.bottomAnchor.constraint(equalTo: contentView.bottomAnchor, constant: -8),

            btnToggleGroups.leadingAnchor.constraint(equalTo: btnSelectTopCPU.trailingAnchor, constant: 4),
            btnToggleGroups.bottomAnchor.constraint(equalTo: contentView.bottomAnchor, constant: -8),

            btnSmartOptimize.leadingAnchor.constraint(equalTo: btnToggleGroups.trailingAnchor, constant: 4),
            btnSmartOptimize.bottomAnchor.constraint(equalTo: contentView.bottomAnchor, constant: -8),

            spinner.leadingAnchor.constraint(equalTo: btnSmartOptimize.trailingAnchor, constant: 4),
            spinner.centerYAnchor.constraint(equalTo: btnSmartOptimize.centerYAnchor),

            commandPopup.trailingAnchor.constraint(equalTo: btnCancel.leadingAnchor, constant: -6),
            commandPopup.bottomAnchor.constraint(equalTo: contentView.bottomAnchor, constant: -8),
            commandPopup.widthAnchor.constraint(equalToConstant: 170),

            btnClose.trailingAnchor.constraint(equalTo: contentView.trailingAnchor, constant: -8),
            btnClose.bottomAnchor.constraint(equalTo: contentView.bottomAnchor, constant: -8),

            btnCancel.trailingAnchor.constraint(equalTo: btnClose.leadingAnchor, constant: -4),
            btnCancel.bottomAnchor.constraint(equalTo: contentView.bottomAnchor, constant: -8),
        ])

        updateProfileHint()
        updateInspector()
    }

    private func addCommandMenuItem(_ title: String, command: String) {
        commandPopup.addItem(withTitle: title)
        commandPopup.lastItem?.representedObject = command
    }

    // MARK: - Data Loading

    func loadData(from file: String) -> Bool {
        guard let data = FileManager.default.contents(atPath: file) else {
            fputs(LF("picker.error.read_file", file), stderr)
            return false
        }

        let decoder = JSONDecoder()
        do {
            let processData = try decoder.decode(ProcessData.self, from: data)
            applyLoadedData(processData)
            return true
        } catch {
            fputs(LF("picker.error.parse_json", error.localizedDescription), stderr)
            return false
        }
    }

    private func applyLoadedData(_ processData: ProcessData) {
        let enriched = enrichChromeRows(processData)
        viewModel.load(from: enriched)
        systemHealth = enriched.system
        persistUsageSnapshot(enriched)
        if let health = systemHealth {
            summaryView.update(health: health)
        }
        updateStatus()
    }

    private func persistUsageSnapshot(_ processData: ProcessData) {
        let fm = FileManager.default
        let logDir = NSHomeDirectory() + "/.local/log/macmon"
        let file = logDir + "/usage-history.jsonl"
        try? fm.createDirectory(atPath: logDir, withIntermediateDirectories: true)
        let topRAM = processData.processes.max(by: { $0.ramMB < $1.ramMB })
        let topCPU = processData.processes.max(by: { $0.cpuPct < $1.cpuPct })
        let record: [String: Any] = [
            "ts": Int(Date().timeIntervalSince1970),
            "freePercent": processData.system.freePercent,
            "swapUsedMB": processData.system.swapUsedMB,
            "totalProcesses": processData.system.totalProcesses,
            "monitored": processData.system.monitoredCount,
            "idle": processData.system.idleCount,
            "topRAM": topRAM?.name ?? "",
            "topRAMMB": topRAM?.ramMB ?? 0,
            "topCPU": topCPU?.name ?? "",
            "topCPUPct": topCPU?.cpuPct ?? 0,
        ]
        guard let data = try? JSONSerialization.data(withJSONObject: record),
              let line = String(data: data, encoding: .utf8) else { return }
        if !fm.fileExists(atPath: file) {
            fm.createFile(atPath: file, contents: nil)
        }
        if let fh = FileHandle(forWritingAtPath: file) {
            fh.seekToEndOfFile()
            if let out = (line + "\n").data(using: .utf8) {
                fh.write(out)
            }
            try? fh.close()
        }
    }

    private func enrichChromeRows(_ data: ProcessData) -> ProcessData {
        guard let tabs = fetchChromeTabs(), !tabs.isEmpty else { return data }
        var tabsByID: [Int: (title: String, url: String)] = [:]
        for tab in tabs {
            if let id = tab.id {
                tabsByID[id] = (title: tab.title, url: tab.url)
            }
        }
        let fallbackTabs = tabs.map { (title: $0.title, url: $0.url) }.filter { !$0.url.isEmpty || !$0.title.isEmpty }
        var fallbackIndex = 0
        let enriched = data.processes.map { p -> ProcessEntry in
            guard p.name == "Chrome Tab" else { return p }

            var tab: (title: String, url: String)? = nil
            if let tabID = extractTabID(from: p.detail), let exact = tabsByID[tabID] {
                tab = exact
            } else if fallbackIndex < fallbackTabs.count {
                tab = fallbackTabs[fallbackIndex]
                fallbackIndex += 1
            }
            guard let tabInfo = tab else { return p }

            let domain = domainFromURL(tabInfo.url)
            let betterDetail: String
            if !tabInfo.title.isEmpty && !tabInfo.url.isEmpty {
                betterDetail = "\(tabInfo.title) — \(tabInfo.url)"
            } else if !tabInfo.title.isEmpty && !domain.isEmpty {
                betterDetail = "\(tabInfo.title) [\(domain)]"
            } else if !tabInfo.title.isEmpty {
                betterDetail = tabInfo.title
            } else if !tabInfo.url.isEmpty {
                betterDetail = tabInfo.url
            } else {
                betterDetail = p.detail
            }
            let betterGroup = domain.isEmpty ? p.group : "Chrome: \(domain)"
            let betterCWD = tabInfo.url.isEmpty ? p.cwd : tabInfo.url

            return ProcessEntry(
                pid: p.pid,
                name: p.name,
                execName: p.execName,
                ramMB: p.ramMB,
                cpuPct: p.cpuPct,
                uptime: p.uptime,
                uptimeSeconds: p.uptimeSeconds,
                cwd: betterCWD,
                tty: p.tty,
                idle: p.idle,
                detail: betterDetail,
                group: betterGroup,
                isSystem: p.isSystem,
                state: p.state,
                diskReadMB: p.diskReadMB,
                diskWriteMB: p.diskWriteMB
            )
        }
        return ProcessData(processes: enriched, system: data.system)
    }

    private func fetchChromeTabs() -> [(id: Int?, title: String, url: String)]? {
        guard let home = ProcessInfo.processInfo.environment["MACMON_HOME"] else { return nil }
        let script = home + "/scripts/chrome-tabs.sh"
        guard FileManager.default.isExecutableFile(atPath: script) else { return nil }
        let task = Process()
        task.executableURL = URL(fileURLWithPath: script)
        task.arguments = ["--json"]
        task.standardError = FileHandle.nullDevice
        let pipe = Pipe()
        task.standardOutput = pipe
        do {
            try task.run()
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            task.waitUntilExit()
            guard let json = try JSONSerialization.jsonObject(with: data) as? [[String: Any]] else { return nil }
            var out: [(id: Int?, title: String, url: String)] = []
            for item in json {
                let idRaw = (item["id"] as? String) ?? ""
                let id = Int(idRaw)
                let title = (item["title"] as? String) ?? ""
                let url = (item["url"] as? String) ?? ""
                out.append((id: id, title: title, url: url))
            }
            return out
        } catch {
            return nil
        }
    }

    private func extractTabID(from detail: String) -> Int? {
        guard let r = detail.range(of: "Tab ID:\\s*([0-9]+)", options: .regularExpression) else { return nil }
        let token = String(detail[r]).components(separatedBy: CharacterSet.decimalDigits.inverted).joined()
        return Int(token)
    }

    private func domainFromURL(_ url: String) -> String {
        guard let u = URL(string: url), let host = u.host else { return "" }
        if host.hasPrefix("www.") { return String(host.dropFirst(4)) }
        return host
    }

    func loadDataAsync(from file: String, completion: @escaping (Bool) -> Void) {
        dataQueue.async { [weak self] in
            guard let self = self else {
                DispatchQueue.main.async { completion(false) }
                return
            }
            guard let data = FileManager.default.contents(atPath: file) else {
                DispatchQueue.main.async {
                    fputs(LF("picker.error.read_file", file), stderr)
                    completion(false)
                }
                return
            }
            let decoder = JSONDecoder()
            let decoded: ProcessData
            do {
                decoded = try decoder.decode(ProcessData.self, from: data)
            } catch {
                DispatchQueue.main.async {
                    fputs(LF("picker.error.parse_json", error.localizedDescription), stderr)
                    completion(false)
                }
                return
            }
            DispatchQueue.main.async {
                self.applyLoadedData(decoded)
                completion(true)
            }
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
                cell.stringValue = "\(arrow) " + LF("picker.group.header", name, count, totalRAM)
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
        cb.setAccessibilityLabel("\(proc.name) PID \(proc.pid)")
        cb.translatesAutoresizingMaskIntoConstraints = false
        wrapper.addSubview(cb)
        NSLayoutConstraint.activate([
            cb.centerXAnchor.constraint(equalTo: wrapper.centerXAnchor),
            cb.centerYAnchor.constraint(equalTo: wrapper.centerYAnchor),
        ])
        return wrapper
    }

    private func idleCell(_ tableView: NSTableView, idle: Bool) -> NSView {
        let value = idle ? L("picker.idle.yes") : L("picker.idle.no")
        if let existing = tableView.makeView(withIdentifier: CellIdle, owner: self) as? NSTextField {
            existing.stringValue = value
            existing.textColor = idle ? .systemBlue : .tertiaryLabelColor
            return existing
        }
        let cell = NSTextField(labelWithString: value)
        cell.identifier = CellIdle
        cell.alignment = .center
        cell.font = NSFont.systemFont(ofSize: 11, weight: .medium)
        cell.textColor = idle ? .systemBlue : .tertiaryLabelColor
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
            let shownName: String
            if proc.name.count <= 2 && proc.execName.count > proc.name.count {
                shownName = proc.execName
            } else {
                shownName = proc.name
            }
            cell.stringValue = shownName
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
            } else if proc.cwd.hasPrefix("http://") || proc.cwd.hasPrefix("https://") {
                cell.stringValue = proc.cwd
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

    @objc func selectTopRAM(_ sender: Any?) {
        var selected = 0
        for i in viewModel.filteredIndices.sorted(by: { viewModel.allProcesses[$0].ramMB > viewModel.allProcesses[$1].ramMB }) {
            if viewModel.allProcesses[i].isSystem { continue }
            viewModel.allProcesses[i].selected = true
            selected += 1
            if selected >= 5 { break }
        }
        tableView.reloadData()
        updateStatus()
    }

    @objc func selectTopCPU(_ sender: Any?) {
        var selected = 0
        for i in viewModel.filteredIndices.sorted(by: { viewModel.allProcesses[$0].cpuPct > viewModel.allProcesses[$1].cpuPct }) {
            if viewModel.allProcesses[i].isSystem { continue }
            viewModel.allProcesses[i].selected = true
            selected += 1
            if selected >= 5 { break }
        }
        tableView.reloadData()
        updateStatus()
    }

    @objc func toggleGrouping(_ sender: Any?) {
        viewModel.groupingEnabled.toggle()
        viewModel.applyFilterAndSort()
        tableView.reloadData()
    }

    @objc func profileChanged(_ sender: NSPopUpButton) {
        guard let profile = sender.titleOfSelectedItem else { return }
        updateProfileHint()
        switchProfile(profile)
    }

    @objc func filterToggled(_ sender: NSButton) {
        viewModel.hideSystemProcesses = (hideSystemCheckbox.state == .on)
        viewModel.showOnlyIdle = (idleOnlyCheckbox.state == .on)
        viewModel.applyFilterAndSort()
        tableView.reloadData()
        updateStatus()
    }

    @objc func commandSelected(_ sender: NSPopUpButton) {
        guard let item = sender.selectedItem,
              let command = item.representedObject as? String,
              !command.isEmpty else {
            sender.selectItem(at: 0)
            return
        }

        performCommand(command)
        sender.selectItem(at: 0)
    }

    func performCommand(_ command: String) {
        switch command {
        case "open_config":
            openConfigEditorWindow()
        case "edit_config_gui":
            openConfigEditorWindow()
        case "preferences":
            openPreferencesWindow()
        case "chrome_tabs":
            showChromeTabsOverview()
        case "reset_config":
            let out = runCLI(args: ["config", "reset"])
            showCommandResult(out)
            reloadProfiles()
        case "log_tail":
            showCommandResult(runCLI(args: ["log"]))
        case "start":
            showCommandResult(runCLI(args: ["start"]))
        case "stop":
            showCommandResult(runCLI(args: ["stop"]))
        case "restart":
            showCommandResult(runCLI(args: ["restart"]))
        case "export_json":
            showCommandResult(runCLI(args: ["export", "json"]))
        case "export_csv":
            showCommandResult(runCLI(args: ["export", "csv"]))
        case "status":
            showCommandResult(runCLI(args: ["status"]))
        case "version":
            showCommandResult(runCLI(args: ["version"]))
        case "help":
            showCommandResult(runCLI(args: ["help"]))
        case "update":
            showCommandResult(runCLI(args: ["update"]))
        default:
            break
        }
    }

    @objc func menuOpenConfig(_ sender: Any?) { performCommand("open_config") }
    @objc func menuOpenConfigEditor(_ sender: Any?) { performCommand("edit_config_gui") }
    @objc func menuOpenPreferences(_ sender: Any?) { performCommand("preferences") }
    @objc func menuShowChromeTabs(_ sender: Any?) { performCommand("chrome_tabs") }
    @objc func menuShowAbout(_ sender: Any?) {
        let alert = NSAlert()
        alert.messageText = L("about.title")
        let version = runCLI(args: ["version"]).replacingOccurrences(of: "macmon v", with: "").trimmingCharacters(in: .whitespacesAndNewlines)
        alert.informativeText = LF("about.body", version)
        alert.addButton(withTitle: L("statusbar.alert.ok"))
        alert.runModal()
    }
    @objc func menuResetConfig(_ sender: Any?) { performCommand("reset_config") }
    @objc func menuRestartDaemon(_ sender: Any?) { performCommand("restart") }
    @objc func menuExportJSON(_ sender: Any?) { performCommand("export_json") }
    @objc func menuExportCSV(_ sender: Any?) { performCommand("export_csv") }
    @objc func menuUpdate(_ sender: Any?) { performCommand("update") }
    @objc func menuFocusSearch(_ sender: Any?) { window.makeFirstResponder(searchField) }
    @objc func menuToggleHideSystem(_ sender: Any?) {
        hideSystemCheckbox.state = (hideSystemCheckbox.state == .on) ? .off : .on
        filterToggled(hideSystemCheckbox)
    }
    @objc func menuToggleIdleOnly(_ sender: Any?) {
        idleOnlyCheckbox.state = (idleOnlyCheckbox.state == .on) ? .off : .on
        filterToggled(idleOnlyCheckbox)
    }

    private func stripANSI(_ text: String) -> String {
        guard let regex = try? NSRegularExpression(pattern: "\\u{001B}\\[[0-9;]*[A-Za-z]", options: []) else {
            return text
        }
        let range = NSRange(location: 0, length: (text as NSString).length)
        return regex.stringByReplacingMatches(in: text, options: [], range: range, withTemplate: "")
    }

    private func openPreferencesWindow() {
        if preferencesWindowController == nil {
            preferencesWindowController = PreferencesWindowController()
        }
        preferencesWindowController?.showWindow(nil)
        preferencesWindowController?.window?.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }

    private func showChromeTabsOverview() {
        guard let home = ProcessInfo.processInfo.environment["MACMON_HOME"] else {
            showCommandResult(L("chrome.tabs.unavailable"))
            return
        }
        let script = home + "/scripts/chrome-tabs.sh"
        guard FileManager.default.isExecutableFile(atPath: script) else {
            showCommandResult(L("chrome.tabs.unavailable"))
            return
        }
        let task = Process()
        task.executableURL = URL(fileURLWithPath: script)
        task.arguments = ["--json"]
        task.standardError = FileHandle.nullDevice
        let pipe = Pipe()
        task.standardOutput = pipe
        do {
            try task.run()
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            task.waitUntilExit()
            guard let json = try JSONSerialization.jsonObject(with: data) as? [[String: Any]] else {
                showCommandResult(L("chrome.tabs.unavailable"))
                return
            }
            let normalized = json.compactMap { item -> (id: String, title: String, url: String)? in
                let id = ((item["id"] as? String) ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
                let title = ((item["title"] as? String) ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
                let url = ((item["url"] as? String) ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
                if title.isEmpty && url.isEmpty { return nil }
                return (id: id, title: title, url: url)
            }
            let lines = normalized.prefix(30).map { tab -> String in
                let title = tab.title.isEmpty ? L("chrome.tabs.untitled") : tab.title
                let url = tab.url.isEmpty ? L("chrome.tabs.no_url") : tab.url
                if tab.id.isEmpty {
                    return "• \(title)\n  \(url)"
                }
                return "• [\(tab.id)] \(title)\n  \(url)"
            }
            let body = lines.joined(separator: "\n")
            if body.isEmpty {
                showCommandResult(L("chrome.tabs.empty_help"))
            } else {
                let alert = NSAlert()
                alert.messageText = L("chrome.tabs.title")
                alert.informativeText = body
                alert.addButton(withTitle: L("statusbar.alert.ok"))
                alert.runModal()
            }
        } catch {
            showCommandResult(L("chrome.tabs.unavailable"))
        }
    }

    private func openConfigEditorWindow() {
        if configWindow == nil {
            let rect = NSRect(x: 0, y: 0, width: 780, height: 520)
            let win = NSWindow(contentRect: rect,
                               styleMask: [.titled, .closable, .resizable],
                               backing: .buffered,
                               defer: false)
            win.title = L("config.editor.title")
            let content = NSView(frame: rect)
            win.contentView = content

            let pathLabel = NSTextField(labelWithString: "")
            pathLabel.translatesAutoresizingMaskIntoConstraints = false
            pathLabel.font = NSFont.monospacedSystemFont(ofSize: 11, weight: .regular)
            pathLabel.textColor = .secondaryLabelColor
            pathLabel.tag = 901
            content.addSubview(pathLabel)

            let helper = NSTextField(labelWithString: L("config.editor.helper"))
            helper.translatesAutoresizingMaskIntoConstraints = false
            helper.textColor = .secondaryLabelColor
            helper.font = NSFont.systemFont(ofSize: 11)
            content.addSubview(helper)

            let ramField = makeConfigField(placeholder: L("config.field.ram_free"))
            let swapField = makeConfigField(placeholder: L("config.field.swap_used"))
            let minRAMField = makeConfigField(placeholder: L("config.field.min_ram"))
            let idleCPUField = makeConfigField(placeholder: L("config.field.idle_cpu"))
            let checkField = makeConfigField(placeholder: L("config.field.check"))
            let idleCheckField = makeConfigField(placeholder: L("config.field.idle_check"))
            let cooldownField = makeConfigField(placeholder: L("config.field.cooldown"))
            let graceField = makeConfigField(placeholder: L("config.field.kill_grace"))
            let diskIO = NSButton(checkboxWithTitle: L("config.field.disk_io"), target: nil, action: nil)
            diskIO.translatesAutoresizingMaskIntoConstraints = false
            content.addSubview(diskIO)
            configDiskIOCheckbox = diskIO

            configFields = [
                "ram": ramField,
                "swap": swapField,
                "minram": minRAMField,
                "idlecpu": idleCPUField,
                "check": checkField,
                "idlecheck": idleCheckField,
                "cooldown": cooldownField,
                "grace": graceField,
            ]

            let scroll = NSScrollView(frame: .zero)
            scroll.translatesAutoresizingMaskIntoConstraints = false
            scroll.hasVerticalScroller = true
            scroll.hasHorizontalScroller = true
            let text = NSTextView(frame: .zero)
            text.isRichText = false
            text.isAutomaticQuoteSubstitutionEnabled = false
            text.font = NSFont.monospacedSystemFont(ofSize: 12, weight: .regular)
            scroll.documentView = text
            content.addSubview(scroll)
            configTextView = text

            let reloadBtn = NSButton(title: L("config.editor.reload"), target: self, action: #selector(reloadConfigEditor))
            let saveBtn = NSButton(title: L("config.editor.save"), target: self, action: #selector(saveConfigEditor))
            let resetBtn = NSButton(title: L("config.editor.reset"), target: self, action: #selector(resetConfigEditor))
            let applyGuidedBtn = NSButton(title: L("config.editor.apply_guided"), target: self, action: #selector(applyGuidedConfig))
            for btn in [reloadBtn, saveBtn, resetBtn] {
                btn.translatesAutoresizingMaskIntoConstraints = false
                content.addSubview(btn)
            }
            applyGuidedBtn.translatesAutoresizingMaskIntoConstraints = false
            content.addSubview(applyGuidedBtn)

            NSLayoutConstraint.activate([
                pathLabel.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 12),
                pathLabel.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -12),
                pathLabel.topAnchor.constraint(equalTo: content.topAnchor, constant: 10),

                helper.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 12),
                helper.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -12),
                helper.topAnchor.constraint(equalTo: pathLabel.bottomAnchor, constant: 4),

                ramField.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 12),
                ramField.topAnchor.constraint(equalTo: helper.bottomAnchor, constant: 8),
                ramField.widthAnchor.constraint(equalToConstant: 170),

                swapField.leadingAnchor.constraint(equalTo: ramField.trailingAnchor, constant: 8),
                swapField.topAnchor.constraint(equalTo: ramField.topAnchor),
                swapField.widthAnchor.constraint(equalToConstant: 170),

                minRAMField.leadingAnchor.constraint(equalTo: swapField.trailingAnchor, constant: 8),
                minRAMField.topAnchor.constraint(equalTo: ramField.topAnchor),
                minRAMField.widthAnchor.constraint(equalToConstant: 180),

                idleCPUField.leadingAnchor.constraint(equalTo: minRAMField.trailingAnchor, constant: 8),
                idleCPUField.topAnchor.constraint(equalTo: ramField.topAnchor),
                idleCPUField.widthAnchor.constraint(equalToConstant: 120),

                checkField.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 12),
                checkField.topAnchor.constraint(equalTo: ramField.bottomAnchor, constant: 8),
                checkField.widthAnchor.constraint(equalToConstant: 170),

                idleCheckField.leadingAnchor.constraint(equalTo: checkField.trailingAnchor, constant: 8),
                idleCheckField.topAnchor.constraint(equalTo: checkField.topAnchor),
                idleCheckField.widthAnchor.constraint(equalToConstant: 170),

                cooldownField.leadingAnchor.constraint(equalTo: idleCheckField.trailingAnchor, constant: 8),
                cooldownField.topAnchor.constraint(equalTo: checkField.topAnchor),
                cooldownField.widthAnchor.constraint(equalToConstant: 170),

                graceField.leadingAnchor.constraint(equalTo: cooldownField.trailingAnchor, constant: 8),
                graceField.topAnchor.constraint(equalTo: checkField.topAnchor),
                graceField.widthAnchor.constraint(equalToConstant: 120),

                diskIO.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 12),
                diskIO.topAnchor.constraint(equalTo: checkField.bottomAnchor, constant: 8),

                applyGuidedBtn.leadingAnchor.constraint(equalTo: diskIO.trailingAnchor, constant: 10),
                applyGuidedBtn.centerYAnchor.constraint(equalTo: diskIO.centerYAnchor),

                scroll.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 12),
                scroll.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -12),
                scroll.topAnchor.constraint(equalTo: diskIO.bottomAnchor, constant: 8),
                scroll.bottomAnchor.constraint(equalTo: saveBtn.topAnchor, constant: -10),

                reloadBtn.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 12),
                reloadBtn.bottomAnchor.constraint(equalTo: content.bottomAnchor, constant: -10),

                resetBtn.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -12),
                resetBtn.bottomAnchor.constraint(equalTo: content.bottomAnchor, constant: -10),

                saveBtn.trailingAnchor.constraint(equalTo: resetBtn.leadingAnchor, constant: -8),
                saveBtn.bottomAnchor.constraint(equalTo: content.bottomAnchor, constant: -10),
            ])

            configWindow = win
        }

        loadConfigIntoEditor()
        configWindow?.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }

    private func loadConfigIntoEditor() {
        let path = runCLI(args: ["config", "path"]).trimmingCharacters(in: .whitespacesAndNewlines)
        guard !path.isEmpty else { return }
        configPath = path
        if let label = configWindow?.contentView?.viewWithTag(901) as? NSTextField {
            label.stringValue = path
        }
        let text = (try? String(contentsOfFile: path, encoding: .utf8)) ?? runCLI(args: ["config"])
        configTextView?.string = text
        populateGuidedFields(from: text)
    }

    private func makeConfigField(placeholder: String) -> NSTextField {
        let f = NSTextField(string: "")
        f.translatesAutoresizingMaskIntoConstraints = false
        f.placeholderString = placeholder
        if let cell = f.cell as? NSTextFieldCell {
            cell.usesSingleLineMode = true
            cell.lineBreakMode = .byTruncatingTail
        }
        configWindow?.contentView?.addSubview(f)
        return f
    }

    private func populateGuidedFields(from yaml: String) {
        let s = ConfigQuickSettings.parse(from: yaml)
        configFields["ram"]?.stringValue = String(s.ramFreePercent)
        configFields["swap"]?.stringValue = String(s.swapUsedMB)
        configFields["minram"]?.stringValue = String(s.processMinRAMKB)
        configFields["idlecpu"]?.stringValue = String(format: "%.2f", s.idleCPUPercent)
        configFields["check"]?.stringValue = String(s.checkIntervalSec)
        configFields["idlecheck"]?.stringValue = String(s.idleCheckSec)
        configFields["cooldown"]?.stringValue = String(s.cooldownSec)
        configFields["grace"]?.stringValue = String(s.killGraceSec)
        configDiskIOCheckbox?.state = s.collectDiskIO ? .on : .off
    }

    @objc private func applyGuidedConfig(_ sender: Any?) {
        var s = ConfigQuickSettings()
        s.ramFreePercent = Int(configFields["ram"]?.stringValue ?? "") ?? s.ramFreePercent
        s.swapUsedMB = Int(configFields["swap"]?.stringValue ?? "") ?? s.swapUsedMB
        s.processMinRAMKB = Int(configFields["minram"]?.stringValue ?? "") ?? s.processMinRAMKB
        s.idleCPUPercent = Double(configFields["idlecpu"]?.stringValue ?? "") ?? s.idleCPUPercent
        s.checkIntervalSec = Int(configFields["check"]?.stringValue ?? "") ?? s.checkIntervalSec
        s.idleCheckSec = Int(configFields["idlecheck"]?.stringValue ?? "") ?? s.idleCheckSec
        s.cooldownSec = Int(configFields["cooldown"]?.stringValue ?? "") ?? s.cooldownSec
        s.killGraceSec = Int(configFields["grace"]?.stringValue ?? "") ?? s.killGraceSec
        s.collectDiskIO = (configDiskIOCheckbox?.state == .on)
        configTextView?.string = s.renderYAML()
    }

    @objc private func reloadConfigEditor(_ sender: Any?) {
        loadConfigIntoEditor()
    }

    @objc private func saveConfigEditor(_ sender: Any?) {
        guard !configPath.isEmpty, let content = configTextView?.string else { return }
        do {
            try content.write(toFile: configPath, atomically: true, encoding: .utf8)
            _ = runCLI(args: ["profile", "current"])
            showCommandResult(L("config.editor.saved"))
            reloadProfiles()
        } catch {
            let msg = L("config.editor.error") + ": " + error.localizedDescription
            showCommandResult(msg)
        }
    }

    @objc private func resetConfigEditor(_ sender: Any?) {
        showCommandResult(runCLI(args: ["config", "reset"]))
        loadConfigIntoEditor()
        reloadProfiles()
    }

    @objc func showSummaryInsights(_ sender: Any?) {
        guard let health = systemHealth else { return }
        let recommendation: String
        if health.freePercent < 20 {
            recommendation = L("picker.metrics.rec.low_ram")
        } else if health.swapUsedMB > 3072 {
            recommendation = L("picker.metrics.rec.swap")
        } else if health.idleCount > 50 {
            recommendation = L("picker.metrics.rec.idle")
        } else {
            recommendation = L("picker.metrics.rec.healthy")
        }

        let alert = NSAlert()
        alert.messageText = L("picker.metrics.title")
        alert.informativeText = LF("picker.metrics.body", health.freePercent, health.swapUsedMB, health.totalProcesses, health.monitoredCount, health.idleCount, recommendation)
        alert.addButton(withTitle: L("statusbar.alert.ok"))
        alert.runModal()
    }

    private func showCommandResult(_ output: String) {
        let cleaned = stripANSI(output).trimmingCharacters(in: .whitespacesAndNewlines)
        guard !cleaned.isEmpty else { return }
        let alert = NSAlert()
        alert.messageText = L("picker.commands.result.title")
        alert.informativeText = cleaned
        alert.addButton(withTitle: L("statusbar.alert.ok"))
        alert.runModal()
    }

    private func reloadProfiles() {
        profilePopup.removeAllItems()
        let output = runCLI(args: ["profile", "list"])
        let profiles = output.split(separator: "\n").map { String($0) }.filter { !$0.isEmpty }
        if profiles.isEmpty {
            profilePopup.addItem(withTitle: L("picker.profile.default_name"))
            updateProfileHint()
            return
        }
        profilePopup.addItems(withTitles: profiles)
        let currentRaw = runCLI(args: ["profile", "current"])
        if let idx = currentRaw.lastIndex(of: ":") {
            let current = currentRaw[currentRaw.index(after: idx)...].trimmingCharacters(in: .whitespacesAndNewlines)
            profilePopup.selectItem(withTitle: current)
        }
        updateProfileHint()
    }

    private func updateProfileHint() {
        let profile = profilePopup.titleOfSelectedItem ?? "default"
        switch profile {
        case "developer":
            profileHintLabel.stringValue = L("picker.profile.developer")
        case "creator":
            profileHintLabel.stringValue = L("picker.profile.creator")
        case "gaming-performance":
            profileHintLabel.stringValue = L("picker.profile.gaming")
        default:
            profileHintLabel.stringValue = L("picker.profile.default")
        }
    }

    private func switchProfile(_ profile: String) {
        dataQueue.async { [weak self] in
            _ = self?.runCLI(args: ["profile", "use", profile])
        }
    }

    private func runCLI(args: [String]) -> String {
        guard let home = ProcessInfo.processInfo.environment["MACMON_HOME"] else { return "" }
        let cliPath = home + "/src/cli/macmon.sh"
        guard FileManager.default.isExecutableFile(atPath: cliPath) else { return "" }
        let task = Process()
        task.executableURL = URL(fileURLWithPath: cliPath)
        task.arguments = args
        var env = ProcessInfo.processInfo.environment
        env["MACMON_HOME"] = home
        task.environment = env
        let pipe = Pipe()
        task.standardOutput = pipe
        task.standardError = FileHandle.nullDevice
        do {
            try task.run()
            // Read data BEFORE waitUntilExit to avoid deadlock when pipe buffer fills
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            task.waitUntilExit()
            return String(data: data, encoding: .utf8) ?? ""
        } catch {
            return ""
        }
    }

    @objc func smartOptimize(_ sender: Any?) {
        guard !isAnalyzingAI else { return }
        isAnalyzingAI = true
        smartOptimizeButton?.isEnabled = false
        smartOptimizeButton?.title = L("picker.ai.analyzing")
        smartOptimizeButton?.setAccessibilityHelp(L("picker.ai.a11y.busy"))
        aiSpinner?.startAnimation(nil)
        NSAccessibility.post(element: smartOptimizeButton as Any, notification: .valueChanged)

        let providerRaw = UserDefaults.standard.string(forKey: "macmon.ai.provider") ?? AIProvider.openai.rawValue
        let model = UserDefaults.standard.string(forKey: "macmon.ai.model") ?? "gpt-4o-mini"
        let provider = AIProvider(rawValue: providerRaw) ?? .openai
        let currentProfile = profilePopup.titleOfSelectedItem ?? "default"

        let top = viewModel.allProcesses
            .filter { !$0.isSystem && !aiBlockedNames.contains($0.name) }
            .sorted { ($0.cpuPct + $0.ramMB / 1024.0) > ($1.cpuPct + $1.ramMB / 1024.0) }
            .prefix(50)

        let summary: [[String: Any]] = top.map {
            ["pid": $0.pid, "name": $0.name, "cpuPct": $0.cpuPct, "ramMB": $0.ramMB, "isSystem": $0.isSystem]
        }

        AIService.shared.analyzeTopProcesses(provider: provider, model: model, profile: currentProfile, processSummary: summary) { [weak self] result in
            guard let self = self else { return }
            DispatchQueue.main.async {
                self.isAnalyzingAI = false
                self.smartOptimizeButton?.isEnabled = true
                self.smartOptimizeButton?.title = L("picker.button.smart_optimize")
                self.smartOptimizeButton?.setAccessibilityHelp(L("picker.ai.a11y.button_help"))
                self.aiSpinner?.stopAnimation(nil)
                switch result {
                case .failure(let error):
                    self.presentAIError(error.localizedDescription)
                case .success(let suggested):
                    self.presentAISuggestions(suggested)
                }
            }
        }
    }

    private func presentAIError(_ message: String) {
        let alert = NSAlert()
        alert.messageText = L("picker.ai.error_title")
        alert.informativeText = message
        alert.addButton(withTitle: L("statusbar.alert.ok"))
        alert.runModal()
    }

    private func presentAISuggestions(_ pids: [Int]) {
        var processTable: [Int: String] = [:]
        for p in viewModel.allProcesses where !p.isSystem {
            processTable[p.pid] = p.name
        }
        let safePIDs = AIService.sanitizeSuggestedPIDs(pids, processTable: processTable)
        if safePIDs.isEmpty {
            presentAIError(L("picker.ai.no_safe"))
            return
        }

        for i in 0..<viewModel.allProcesses.count {
            if safePIDs.contains(viewModel.allProcesses[i].pid) {
                viewModel.allProcesses[i].selected = true
            }
        }
        tableView.reloadData()
        updateStatus()

        let alert = NSAlert()
        alert.messageText = L("picker.ai.review_title")
        let names = viewModel.allProcesses
            .filter { safePIDs.contains($0.pid) }
            .map { p -> String in
                var reasons: [String] = []
                if p.ramMB >= 1024 { reasons.append(String(format: L("picker.ai.reason.ram"), p.ramMB)) }
                if p.cpuPct >= 20 { reasons.append(String(format: L("picker.ai.reason.cpu"), p.cpuPct)) }
                if p.idle { reasons.append(L("picker.ai.reason.idle")) }
                if p.name == "Chrome Tab" { reasons.append(L("picker.ai.reason.chrome")) }
                if reasons.isEmpty { reasons.append(L("picker.ai.reason.generic")) }
                return "\(p.name) (PID \(p.pid)) — " + reasons.joined(separator: ", ")
            }
            .joined(separator: "\n")
        alert.informativeText = names + "\n\n" + L("picker.ai.review_hint")
        let applyBtn = alert.addButton(withTitle: L("picker.ai.apply"))
        applyBtn.setAccessibilityHelp(L("picker.ai.a11y.apply_help"))
        let reviewBtn = alert.addButton(withTitle: L("picker.ai.review"))
        reviewBtn.setAccessibilityHelp(L("picker.ai.a11y.review_help"))
        if alert.runModal() == .alertFirstButtonReturn {
            closeSelected(nil)
        }
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
        // Output selected processes as JSON to stdout
        let selected = viewModel.allProcesses
            .filter { $0.selected }
            .map { ["pid": $0.pid, "name": $0.execName] }
        if let data = try? JSONSerialization.data(withJSONObject: selected, options: []),
           let json = String(data: data, encoding: .utf8) {
            print(json)
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

    func tableViewSelectionDidChange(_ notification: Notification) {
        updateInspector()
    }

    private func updateInspector() {
        guard tableView != nil else { return }
        let row = tableView.selectedRow
        guard row >= 0, row < viewModel.displayRows.count else {
            inspectorLabel.stringValue = L("picker.inspector.empty")
            return
        }
        guard case .process(let idx) = viewModel.displayRows[row], idx < viewModel.allProcesses.count else {
            inspectorLabel.stringValue = L("picker.inspector.group")
            return
        }
        let p = viewModel.allProcesses[idx]
        let detail = p.detail.isEmpty ? "-" : p.detail
        let cwd = p.cwd.isEmpty ? "-" : p.cwd
        let group = p.group.isEmpty ? "-" : p.group
        let idleText = p.idle ? L("picker.idle.yes") : L("picker.idle.no")
        inspectorLabel.stringValue = LF("picker.inspector.value", p.name, p.pid, p.ramMB, p.cpuPct, p.uptime, group, idleText, p.tty, detail, cwd)
    }

    func updateStatus() {
        if !Thread.isMainThread {
            DispatchQueue.main.async { [weak self] in
                self?.updateStatus()
            }
            return
        }
        let total = viewModel.filteredIndices.count
        let selected = viewModel.selectedCount
        let ramTotal = viewModel.selectedRAM
        let idleCount = viewModel.allProcesses.filter { $0.idle }.count

        var text = LF("picker.status.processes", total)
        if !viewModel.searchText.isEmpty {
            text += LF("picker.status.filtered", viewModel.allProcesses.count)
        }
        text += LF("picker.status.idle", idleCount)
        if selected > 0 {
            text += LF("picker.status.selected", selected, ramTotal)
        }
        statusLabel.stringValue = text
        helperLabel.stringValue = buildInsightLine()
        updateInspector()
    }

    private func buildInsightLine() -> String {
        let topRAM = viewModel.allProcesses.max(by: { $0.ramMB < $1.ramMB })
        let topCPU = viewModel.allProcesses.max(by: { $0.cpuPct < $1.cpuPct })
        let chromeTabs = viewModel.allProcesses.filter { $0.name == "Chrome Tab" }.count
        let idleCount = viewModel.allProcesses.filter { $0.idle }.count
        let ramName = topRAM?.name ?? "-"
        let ramMB = topRAM?.ramMB ?? 0
        let cpuName = topCPU?.name ?? "-"
        let cpuPct = topCPU?.cpuPct ?? 0
        return LF("picker.insight.line", ramName, ramMB, cpuName, cpuPct, chromeTabs, idleCount)
    }
}

// MARK: - App Delegate

class AppDelegate: NSObject, NSApplicationDelegate {
    let controller = ProcessPickerController()
    var inputFile: String?
    private var keyboardMonitor: Any?

    private func setupMainMenu() {
        let mainMenu = NSMenu()

        let appItem = NSMenuItem()
        mainMenu.addItem(appItem)
        let appMenu = NSMenu()
        appItem.submenu = appMenu
        let about = NSMenuItem(title: L("menu.app.about"), action: #selector(ProcessPickerController.menuShowAbout(_:)), keyEquivalent: "")
        about.target = controller
        appMenu.addItem(about)
        let prefs = NSMenuItem(title: L("menu.app.preferences"), action: #selector(ProcessPickerController.menuOpenPreferences(_:)), keyEquivalent: ",")
        prefs.target = controller
        appMenu.addItem(prefs)
        appMenu.addItem(NSMenuItem.separator())
        appMenu.addItem(withTitle: L("menu.app.quit"), action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")

        let fileItem = NSMenuItem()
        fileItem.title = L("menu.file")
        mainMenu.addItem(fileItem)
        let fileMenu = NSMenu(title: L("menu.file"))
        fileItem.submenu = fileMenu
        let openConfig = NSMenuItem(title: L("picker.commands.open_config"), action: #selector(ProcessPickerController.menuOpenConfig(_:)), keyEquivalent: "")
        openConfig.target = controller
        fileMenu.addItem(openConfig)
        let editConfig = NSMenuItem(title: L("picker.commands.edit_config_gui"), action: #selector(ProcessPickerController.menuOpenConfigEditor(_:)), keyEquivalent: "")
        editConfig.target = controller
        fileMenu.addItem(editConfig)
        let exportJSON = NSMenuItem(title: L("picker.commands.export_json"), action: #selector(ProcessPickerController.menuExportJSON(_:)), keyEquivalent: "j")
        exportJSON.target = controller
        fileMenu.addItem(exportJSON)
        let exportCSV = NSMenuItem(title: L("picker.commands.export_csv"), action: #selector(ProcessPickerController.menuExportCSV(_:)), keyEquivalent: "e")
        exportCSV.target = controller
        fileMenu.addItem(exportCSV)

        let editItem = NSMenuItem()
        editItem.title = L("menu.edit")
        mainMenu.addItem(editItem)
        let editMenu = NSMenu(title: L("menu.edit"))
        editItem.submenu = editMenu
        let focusSearch = NSMenuItem(title: L("menu.edit.search"), action: #selector(ProcessPickerController.menuFocusSearch(_:)), keyEquivalent: "f")
        focusSearch.target = controller
        editMenu.addItem(focusSearch)
        let selectAll = NSMenuItem(title: L("picker.button.select_all"), action: #selector(ProcessPickerController.selectAll(_:)), keyEquivalent: "a")
        selectAll.target = controller
        editMenu.addItem(selectAll)
        let selectNone = NSMenuItem(title: L("picker.button.select_none"), action: #selector(ProcessPickerController.selectNone(_:)), keyEquivalent: "")
        selectNone.target = controller
        editMenu.addItem(selectNone)

        let viewItem = NSMenuItem()
        viewItem.title = L("menu.view")
        mainMenu.addItem(viewItem)
        let viewMenu = NSMenu(title: L("menu.view"))
        viewItem.submenu = viewMenu
        let toggleGroup = NSMenuItem(title: L("picker.button.groups"), action: #selector(ProcessPickerController.toggleGrouping(_:)), keyEquivalent: "g")
        toggleGroup.target = controller
        viewMenu.addItem(toggleGroup)
        let hideSystem = NSMenuItem(title: L("picker.filter.hide_system"), action: #selector(ProcessPickerController.menuToggleHideSystem(_:)), keyEquivalent: "h")
        hideSystem.target = controller
        viewMenu.addItem(hideSystem)
        let idleOnly = NSMenuItem(title: L("picker.filter.only_idle"), action: #selector(ProcessPickerController.menuToggleIdleOnly(_:)), keyEquivalent: "i")
        idleOnly.target = controller
        viewMenu.addItem(idleOnly)

        let actionsItem = NSMenuItem()
        actionsItem.title = L("menu.actions")
        mainMenu.addItem(actionsItem)
        let actionsMenu = NSMenu(title: L("menu.actions"))
        actionsItem.submenu = actionsMenu
        let smart = NSMenuItem(title: L("picker.button.smart_optimize"), action: #selector(ProcessPickerController.smartOptimize(_:)), keyEquivalent: "o")
        smart.target = controller
        actionsMenu.addItem(smart)
        let aiSettings = NSMenuItem(title: L("picker.commands.ai_settings"), action: #selector(ProcessPickerController.menuOpenPreferences(_:)), keyEquivalent: "")
        aiSettings.target = controller
        actionsMenu.addItem(aiSettings)
        let tabs = NSMenuItem(title: L("picker.commands.chrome_tabs"), action: #selector(ProcessPickerController.menuShowChromeTabs(_:)), keyEquivalent: "t")
        tabs.target = controller
        actionsMenu.addItem(tabs)
        let restart = NSMenuItem(title: L("picker.commands.daemon_restart"), action: #selector(ProcessPickerController.menuRestartDaemon(_:)), keyEquivalent: "r")
        restart.keyEquivalentModifierMask = [.command, .shift]
        restart.target = controller
        actionsMenu.addItem(restart)
        let update = NSMenuItem(title: L("picker.commands.update"), action: #selector(ProcessPickerController.menuUpdate(_:)), keyEquivalent: "u")
        update.target = controller
        actionsMenu.addItem(update)

        NSApplication.shared.mainMenu = mainMenu
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        setupMainMenu()
        controller.setupWindow()

        guard let file = inputFile else {
            fputs(L("picker.error.no_file"), stderr)
            exit(1)
        }

        controller.loadDataAsync(from: file) { [weak self] ok in
            guard let self = self else { return }
            if !ok {
                exit(1)
            }
            self.controller.tableView.reloadData()
            self.controller.window.makeKeyAndOrderFront(nil)
            self.controller.window.makeFirstResponder(self.controller.searchField)
        }

        // Set up keyboard shortcuts
        keyboardMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard let self = self else { return event }
            let searchFocused = self.controller.window.firstResponder is NSTextView &&
                self.controller.searchField.currentEditor() != nil
            if event.modifierFlags.contains(.command) {
                switch event.charactersIgnoringModifiers {
                case "a":
                    if searchFocused { return event }
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
            if searchFocused { return event }
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
        if let monitor = keyboardMonitor {
            NSEvent.removeMonitor(monitor)
            keyboardMonitor = nil
        }
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
