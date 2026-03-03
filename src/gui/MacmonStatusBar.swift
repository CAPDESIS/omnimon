import Cocoa
import Foundation

// MARK: - Native System Data Collection

/// Collects memory and process info using Mach host_statistics64 and sysctl.
/// No subprocess spawning for the periodic refresh.
struct SystemSnapshot {
    let freePercent: Int
    let totalMemGB: Double
    let swapUsedMB: Int
    let processCount: Int

    static func collect() -> SystemSnapshot {
        let mem = collectMemory()
        let swap = collectSwap()
        let procs = collectProcessCount()
        return SystemSnapshot(
            freePercent: mem.freePercent,
            totalMemGB: mem.totalGB,
            swapUsedMB: swap,
            processCount: procs
        )
    }

    // MARK: Memory via host_statistics64

    private static func collectMemory() -> (freePercent: Int, totalGB: Double) {
        let totalBytes = ProcessInfo.processInfo.physicalMemory
        let totalGB = Double(totalBytes) / (1024.0 * 1024.0 * 1024.0)

        var stats = vm_statistics64()
        var count = mach_msg_type_number_t(
            MemoryLayout<vm_statistics64>.stride / MemoryLayout<integer_t>.stride
        )
        let hostPort = mach_host_self()

        let result = withUnsafeMutablePointer(to: &stats) { ptr in
            ptr.withMemoryRebound(to: integer_t.self, capacity: Int(count)) { intPtr in
                host_statistics64(hostPort, HOST_VM_INFO64, intPtr, &count)
            }
        }

        guard result == KERN_SUCCESS else {
            return (freePercent: 50, totalGB: totalGB)
        }

        let pageSize = UInt64(vm_kernel_page_size)
        let freeBytes = UInt64(stats.free_count) * pageSize
        let inactiveBytes = UInt64(stats.inactive_count) * pageSize
        // "free" in macmon's sense: free + inactive (reclaimable)
        let availableBytes = freeBytes + inactiveBytes
        let freePercent = Int((Double(availableBytes) / Double(totalBytes)) * 100.0)

        return (freePercent: max(0, min(100, freePercent)), totalGB: totalGB)
    }

    // MARK: Swap via sysctl

    private static func collectSwap() -> Int {
        var swapUsage = xsw_usage()
        var size = MemoryLayout<xsw_usage>.size
        let result = sysctlbyname("vm.swapusage", &swapUsage, &size, nil, 0)
        guard result == 0 else { return 0 }
        let usedMB = Int(swapUsage.xsu_used / (1024 * 1024))
        return usedMB
    }

    // MARK: Process count via sysctl

    private static func collectProcessCount() -> Int {
        var mib: [Int32] = [CTL_KERN, KERN_PROC, KERN_PROC_ALL, 0]
        var size: Int = 0

        // First call to get the buffer size
        guard sysctl(&mib, UInt32(mib.count), nil, &size, nil, 0) == 0 else {
            return 0
        }

        let count = size / MemoryLayout<kinfo_proc>.stride
        return count
    }
}

// MARK: - Color for RAM percentage

func ramColor(freePercent: Int) -> NSColor {
    if freePercent >= 40 {
        return .systemGreen
    } else if freePercent >= 20 {
        return .systemYellow
    } else {
        return .systemRed
    }
}

// MARK: - Status Bar Controller

class MacmonStatusBarController: NSObject, NSMenuDelegate {
    private var statusItem: NSStatusItem!
    private var menu: NSMenu!
    private var refreshTimer: Timer?
    private let metricsQueue = DispatchQueue(label: "com.macmon.statusbar.metrics", qos: .utility)
    private let configPath: String
    private var lastConfigMTime: TimeInterval = 0

    // Menu items that get updated
    private var ramItem: NSMenuItem!
    private var swapItem: NSMenuItem!
    private var processItem: NSMenuItem!

    // Latest snapshot
    private var snapshot: SystemSnapshot?

    // MACMON_HOME for locating other components (validated at init)
    private let macmonHome: String

    override init() {
        let candidate: String
        if let envHome = ProcessInfo.processInfo.environment["MACMON_HOME"],
           !envHome.isEmpty {
            candidate = envHome
        } else {
            candidate = NSHomeDirectory() + "/.local/libexec/macmon"
        }
        if let explicitConfig = ProcessInfo.processInfo.environment["MACMON_CONFIG"], !explicitConfig.isEmpty {
            configPath = explicitConfig
        } else {
            configPath = NSHomeDirectory() + "/.config/macmon/macmon.yaml"
        }
        // Security: validate MACMON_HOME — must be an absolute path owned by current user,
        // not a symlink to a different owner's directory, and must not contain shell metacharacters
        macmonHome = MacmonStatusBarController.validateMacmonHome(candidate)
        super.init()
    }

    /// Validate and canonicalize MACMON_HOME to prevent path injection (MITRE T1574).
    /// Returns the validated path, or falls back to the default.
    private static func validateMacmonHome(_ path: String) -> String {
        let fm = FileManager.default
        let defaultPath = NSHomeDirectory() + "/.local/libexec/macmon"

        // Must be absolute path
        guard path.hasPrefix("/") else { return defaultPath }

        // Reject shell metacharacters that could break out of quoting
        let forbidden = CharacterSet(charactersIn: "'\"\\`$!;|&()\n\r")
        guard path.rangeOfCharacter(from: forbidden) == nil else {
            fputs("macmon: WARNING: MACMON_HOME contains forbidden characters, using default\n", stderr)
            return defaultPath
        }

        // Resolve symlinks to canonical path
        let resolved = (path as NSString).resolvingSymlinksInPath

        // Must be a directory
        var isDir: ObjCBool = false
        guard fm.fileExists(atPath: resolved, isDirectory: &isDir), isDir.boolValue else {
            return defaultPath
        }

        // Must be owned by current user (prevents privilege escalation via shared dirs)
        if let attrs = try? fm.attributesOfItem(atPath: resolved),
           let owner = attrs[.ownerAccountID] as? NSNumber {
            if owner.uint32Value != getuid() {
                fputs("macmon: WARNING: MACMON_HOME not owned by current user, using default\n", stderr)
                return defaultPath
            }
        }

        return resolved
    }

    func setup() {
        // Create the status item
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)

        if let button = statusItem.button {
            // Try SF Symbols first (macOS 11+), fall back to text
            if let image = NSImage(systemSymbolName: "memorychip",
                                   accessibilityDescription: L("statusbar.accessibility")) {
                let config = NSImage.SymbolConfiguration(pointSize: 14, weight: .regular)
                button.image = image.withSymbolConfiguration(config)
            } else {
                button.title = "M"
            }
            button.toolTip = L("statusbar.tooltip")
            button.setAccessibilityRole(.button)
            button.setAccessibilityLabel(L("statusbar.accessibility"))
        }

        buildMenu()
        statusItem.menu = menu

        // Initial data collection
        refreshData()

        // Refresh every 30 seconds
        refreshTimer = Timer.scheduledTimer(withTimeInterval: 30.0, repeats: true) { [weak self] _ in
            self?.refreshData()
        }
        // Allow timer to fire even during menu tracking
        if let timer = refreshTimer {
            RunLoop.current.add(timer, forMode: .common)
        }
    }

    private func buildMenu() {
        menu = NSMenu()
        menu.delegate = self
        menu.autoenablesItems = false

        // RAM item (placeholder, updated on refresh)
        ramItem = NSMenuItem(title: L("statusbar.ram.placeholder"), action: nil, keyEquivalent: "")
        ramItem.isEnabled = false
        menu.addItem(ramItem)

        // Swap item
        swapItem = NSMenuItem(title: L("statusbar.swap.placeholder"), action: nil, keyEquivalent: "")
        swapItem.isEnabled = false
        menu.addItem(swapItem)

        // Process count item
        processItem = NSMenuItem(title: L("statusbar.process.placeholder"), action: nil, keyEquivalent: "")
        processItem.isEnabled = false
        menu.addItem(processItem)

        // Separator
        menu.addItem(NSMenuItem.separator())

        // Open Process Picker
        let pickerItem = NSMenuItem(
            title: L("statusbar.menu.open_picker"),
            action: #selector(openProcessPicker),
            keyEquivalent: "p"
        )
        pickerItem.keyEquivalentModifierMask = [.command]
        pickerItem.target = self
        menu.addItem(pickerItem)

        let configItem = NSMenuItem(
            title: L("statusbar.menu.open_config"),
            action: #selector(openConfiguration),
            keyEquivalent: ","
        )
        configItem.keyEquivalentModifierMask = [.command]
        configItem.target = self
        menu.addItem(configItem)

        let reloadItem = NSMenuItem(
            title: L("statusbar.menu.reload_config"),
            action: #selector(reloadConfigurationNow),
            keyEquivalent: "r"
        )
        reloadItem.keyEquivalentModifierMask = [.command]
        reloadItem.target = self
        menu.addItem(reloadItem)

        // Export Snapshot submenu
        let exportItem = NSMenuItem(title: L("statusbar.menu.export"), action: nil, keyEquivalent: "")
        let exportMenu = NSMenu()
        let jsonExport = NSMenuItem(
            title: L("statusbar.menu.export.json"),
            action: #selector(exportJSON),
            keyEquivalent: ""
        )
        jsonExport.target = self
        exportMenu.addItem(jsonExport)

        let csvExport = NSMenuItem(
            title: L("statusbar.menu.export.csv"),
            action: #selector(exportCSV),
            keyEquivalent: ""
        )
        csvExport.target = self
        exportMenu.addItem(csvExport)

        exportItem.submenu = exportMenu
        menu.addItem(exportItem)

        // Separator
        menu.addItem(NSMenuItem.separator())

        // Status...
        let statusActionItem = NSMenuItem(
            title: L("statusbar.menu.status"),
            action: #selector(openStatus),
            keyEquivalent: "s"
        )
        statusActionItem.keyEquivalentModifierMask = [.command]
        statusActionItem.target = self
        menu.addItem(statusActionItem)

        // Separator
        menu.addItem(NSMenuItem.separator())

        // Quit
        let quitItem = NSMenuItem(
            title: L("statusbar.menu.quit"),
            action: #selector(quitApp),
            keyEquivalent: "q"
        )
        quitItem.keyEquivalentModifierMask = [.command]
        quitItem.target = self
        menu.addItem(quitItem)
    }

    // MARK: - Data Refresh

    @objc func refreshData() {
        metricsQueue.async { [weak self] in
            guard let self = self else { return }
            let collected = SystemSnapshot.collect()
            DispatchQueue.main.async { [weak self] in
                guard let self = self else { return }
                self.snapshot = collected
                self.updateMenuItems()
                self.updateStatusBarTitle()
                self.autoReloadIfConfigChanged()
            }
        }
    }

    private func autoReloadIfConfigChanged() {
        guard let attrs = try? FileManager.default.attributesOfItem(atPath: configPath),
              let modified = attrs[.modificationDate] as? Date else { return }
        let ts = modified.timeIntervalSince1970
        if lastConfigMTime == 0 {
            lastConfigMTime = ts
            return
        }
        guard ts > lastConfigMTime else { return }
        lastConfigMTime = ts
        signalDaemonReload()
    }

    private func updateMenuItems() {
        guard let snap = snapshot else { return }

        // RAM item with color
        let ramText = LF("statusbar.ram.value", snap.freePercent, snap.totalMemGB)
        let ramAttr = NSMutableAttributedString(string: ramText)
        let color = ramColor(freePercent: snap.freePercent)
        ramAttr.addAttribute(.foregroundColor, value: color,
                             range: NSRange(location: 0, length: ramAttr.length))
        ramAttr.addAttribute(.font, value: NSFont.menuFont(ofSize: 13),
                             range: NSRange(location: 0, length: ramAttr.length))
        ramItem.attributedTitle = ramAttr

        // Swap item
        swapItem.title = LF("statusbar.swap.value", snap.swapUsedMB)

        // Process count
        processItem.title = LF("statusbar.process.value", snap.processCount)
    }

    private func updateStatusBarTitle() {
        guard let snap = snapshot, let button = statusItem.button else { return }

        // Update the button title with a brief percentage indicator
        // Keep the icon and add a small text suffix
        if button.image != nil {
            button.title = " \(snap.freePercent)%"
        } else {
            button.title = "M \(snap.freePercent)%"
        }
    }

    // MARK: - Menu Actions

    @objc func openProcessPicker() {
        let pickerPath = macmonHome + "/ProcessPicker"
        let macmonBin = findMacmonCLI()

        // Security: use Process argument arrays instead of bash -c string interpolation
        // to prevent shell injection via MACMON_HOME (Finding #3)
        if FileManager.default.isExecutableFile(atPath: macmonBin) {
            let task = Process()
            task.executableURL = URL(fileURLWithPath: macmonBin)
            task.arguments = ["picker"]
            var env = ProcessInfo.processInfo.environment
            env["MACMON_HOME"] = macmonHome
            task.environment = env
            try? task.run()
        } else if FileManager.default.isExecutableFile(atPath: pickerPath) {
            let task = Process()
            task.executableURL = URL(fileURLWithPath: pickerPath)
            var env = ProcessInfo.processInfo.environment
            env["MACMON_HOME"] = macmonHome
            task.environment = env
            try? task.run()
        } else {
            showAlert(
                title: L("statusbar.alert.picker_missing_title"),
                message: LF("statusbar.alert.picker_missing_message", pickerPath)
            )
        }
    }

    @objc func exportJSON() {
        runMacmonExport(format: "json")
    }

    @objc func exportCSV() {
        runMacmonExport(format: "csv")
    }

    @objc func openConfiguration() {
        let url = URL(fileURLWithPath: configPath)
        if FileManager.default.fileExists(atPath: configPath) {
            NSWorkspace.shared.open(url)
        } else {
            showAlert(
                title: L("statusbar.alert.config_missing_title"),
                message: LF("statusbar.alert.config_missing_message", configPath)
            )
        }
    }

    @objc func reloadConfigurationNow() {
        signalDaemonReload()
    }

    private func signalDaemonReload() {
        let pidPath = NSTemporaryDirectory() + "macmond.pid"
        guard let content = try? String(contentsOfFile: pidPath, encoding: .utf8),
              let pid = Int32(content.trimmingCharacters(in: .whitespacesAndNewlines)),
              pid > 1 else {
            return
        }
        _ = Darwin.kill(pid, SIGUSR1)
    }

    private func runMacmonExport(format: String) {
        let macmonBin = findMacmonCLI()
        guard FileManager.default.isExecutableFile(atPath: macmonBin) else {
            showAlert(
                title: L("statusbar.alert.cli_missing_title"),
                message: LF("statusbar.alert.cli_missing_message", macmonBin)
            )
            return
        }

        // Use NSSavePanel to let the user choose where to save
        let panel = NSSavePanel()
        panel.title = L("statusbar.export.panel.title")
        panel.nameFieldStringValue = LF("statusbar.export.panel.filename", format)
        if format == "json" {
            panel.allowedContentTypes = [.json]
        }
        panel.canCreateDirectories = true

        let response = panel.runModal()
        guard response == .OK, let url = panel.url else { return }

        metricsQueue.async { [weak self] in
            guard let self = self else { return }
            let task = Process()
            task.executableURL = URL(fileURLWithPath: macmonBin)
            task.arguments = ["export", format]
            var env = ProcessInfo.processInfo.environment
            env["MACMON_HOME"] = self.macmonHome
            task.environment = env

            let pipe = Pipe()
            task.standardOutput = pipe
            task.standardError = FileHandle.nullDevice

            do {
                try task.run()
                task.waitUntilExit()
                let data = pipe.fileHandleForReading.readDataToEndOfFile()
                try data.write(to: url)
            } catch {
                DispatchQueue.main.async { [weak self] in
                    self?.showAlert(
                        title: L("statusbar.alert.export_failed_title"),
                        message: LF("statusbar.alert.export_failed_message", error.localizedDescription)
                    )
                }
            }
        }
    }

    @objc func openStatus() {
        let macmonBin = findMacmonCLI()
        guard FileManager.default.isExecutableFile(atPath: macmonBin) else {
            showAlert(
                title: L("statusbar.alert.cli_missing_title"),
                message: L("statusbar.alert.status_missing_message")
            )
            return
        }

        // Security: avoid AppleScript string interpolation of MACMON_HOME (Finding #2).
        // Use Process API with argument arrays and pipe output to a temporary file,
        // then open that file in Terminal via a safe, fixed AppleScript command.
        metricsQueue.async {
            let task = Process()
            task.executableURL = URL(fileURLWithPath: macmonBin)
            task.arguments = ["status"]
            var env = ProcessInfo.processInfo.environment
            env["MACMON_HOME"] = self.macmonHome
            task.environment = env

            let pipe = Pipe()
            task.standardOutput = pipe
            task.standardError = FileHandle.nullDevice

            do {
                try task.run()
                task.waitUntilExit()
                let data = pipe.fileHandleForReading.readDataToEndOfFile()
                guard let output = String(data: data, encoding: .utf8), !output.isEmpty else { return }
                let tmpPath = NSTemporaryDirectory() + "macmon-status.txt"
                try output.write(toFile: tmpPath, atomically: true, encoding: .utf8)
                DispatchQueue.main.async {
                    let script = """
                        tell application "Terminal"
                            activate
                            do script "cat '\(tmpPath.replacingOccurrences(of: "'", with: "'\\''"))' ; rm -f '\(tmpPath.replacingOccurrences(of: "'", with: "'\\''"))'"
                        end tell
                    """
                    let appleScript = NSAppleScript(source: script)
                    var errorDict: NSDictionary?
                    appleScript?.executeAndReturnError(&errorDict)
                }
            } catch {
                fputs("Warning: Could not run macmon status: \(error)\n", stderr)
            }
        }
    }

    @objc func quitApp() {
        refreshTimer?.invalidate()
        refreshTimer = nil
        NSApp.terminate(nil)
    }

    deinit {
        refreshTimer?.invalidate()
    }

    // MARK: - Helpers

    private func findMacmonCLI() -> String {
        // Check common locations
        let candidates = [
            macmonHome + "/src/cli/macmon.sh",
            NSHomeDirectory() + "/.local/bin/macmon",
            "/usr/local/bin/macmon",
        ]
        for path in candidates {
            if FileManager.default.isExecutableFile(atPath: path) {
                return path
            }
        }
        return candidates[1] // default to ~/.local/bin/macmon
    }

    private func showAlert(title: String, message: String) {
        if !Thread.isMainThread {
            DispatchQueue.main.async { [weak self] in
                self?.showAlert(title: title, message: message)
            }
            return
        }
        let alert = NSAlert()
        alert.messageText = title
        alert.informativeText = message
        alert.alertStyle = .warning
        alert.addButton(withTitle: L("statusbar.alert.ok"))
        alert.runModal()
    }
}

// MARK: - App Delegate

class StatusBarAppDelegate: NSObject, NSApplicationDelegate {
    let controller = MacmonStatusBarController()

    func applicationDidFinishLaunching(_ notification: Notification) {
        controller.setup()
    }

    func applicationWillTerminate(_ notification: Notification) {
        // Cleanup handled by controller
    }
}

// MARK: - Main

@main
struct MacmonStatusBarApp {
    static func main() {
        let app = NSApplication.shared
        app.setActivationPolicy(.accessory) // Menu bar only, no dock icon
        let delegate = StatusBarAppDelegate()
        app.delegate = delegate
        app.run()
    }
}
