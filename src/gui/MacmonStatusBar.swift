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

    // Menu items that get updated
    private var ramItem: NSMenuItem!
    private var swapItem: NSMenuItem!
    private var processItem: NSMenuItem!

    // Latest snapshot
    private var snapshot: SystemSnapshot?

    // MACMON_HOME for locating other components
    private let macmonHome: String

    override init() {
        if let envHome = ProcessInfo.processInfo.environment["MACMON_HOME"],
           !envHome.isEmpty {
            macmonHome = envHome
        } else {
            // Reasonable default
            macmonHome = NSHomeDirectory() + "/.local/libexec/macmon"
        }
        super.init()
    }

    func setup() {
        // Create the status item
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)

        if let button = statusItem.button {
            // Try SF Symbols first (macOS 11+), fall back to text
            if let image = NSImage(systemSymbolName: "memorychip",
                                   accessibilityDescription: "macmon") {
                let config = NSImage.SymbolConfiguration(pointSize: 14, weight: .regular)
                button.image = image.withSymbolConfiguration(config)
            } else {
                button.title = "M"
            }
            button.toolTip = "macmon - System Monitor"
        }

        buildMenu()
        statusItem.menu = menu

        // Initial data collection
        refreshData()

        // Refresh every 30 seconds
        refreshTimer = Timer.scheduledTimer(
            timeInterval: 30.0,
            target: self,
            selector: #selector(refreshData),
            userInfo: nil,
            repeats: true
        )
        // Allow timer to fire even during menu tracking
        RunLoop.current.add(refreshTimer!, forMode: .common)
    }

    private func buildMenu() {
        menu = NSMenu()
        menu.delegate = self
        menu.autoenablesItems = false

        // RAM item (placeholder, updated on refresh)
        ramItem = NSMenuItem(title: "RAM: --", action: nil, keyEquivalent: "")
        ramItem.isEnabled = false
        menu.addItem(ramItem)

        // Swap item
        swapItem = NSMenuItem(title: "Swap: --", action: nil, keyEquivalent: "")
        swapItem.isEnabled = false
        menu.addItem(swapItem)

        // Process count item
        processItem = NSMenuItem(title: "Processes: --", action: nil, keyEquivalent: "")
        processItem.isEnabled = false
        menu.addItem(processItem)

        // Separator
        menu.addItem(NSMenuItem.separator())

        // Open Process Picker
        let pickerItem = NSMenuItem(
            title: "Open Process Picker",
            action: #selector(openProcessPicker),
            keyEquivalent: "p"
        )
        pickerItem.keyEquivalentModifierMask = [.command]
        pickerItem.target = self
        menu.addItem(pickerItem)

        // Export Snapshot submenu
        let exportItem = NSMenuItem(title: "Export Snapshot...", action: nil, keyEquivalent: "")
        let exportMenu = NSMenu()
        let jsonExport = NSMenuItem(
            title: "JSON",
            action: #selector(exportJSON),
            keyEquivalent: ""
        )
        jsonExport.target = self
        exportMenu.addItem(jsonExport)

        let csvExport = NSMenuItem(
            title: "CSV",
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
            title: "Status...",
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
            title: "Quit macmon",
            action: #selector(quitApp),
            keyEquivalent: "q"
        )
        quitItem.keyEquivalentModifierMask = [.command]
        quitItem.target = self
        menu.addItem(quitItem)
    }

    // MARK: - Data Refresh

    @objc func refreshData() {
        snapshot = SystemSnapshot.collect()
        updateMenuItems()
        updateStatusBarTitle()
    }

    private func updateMenuItems() {
        guard let snap = snapshot else { return }

        // RAM item with color
        let ramText = "RAM: \(snap.freePercent)% free of \(String(format: "%.0f", snap.totalMemGB))GB"
        let ramAttr = NSMutableAttributedString(string: ramText)
        let color = ramColor(freePercent: snap.freePercent)
        ramAttr.addAttribute(.foregroundColor, value: color,
                             range: NSRange(location: 0, length: ramAttr.length))
        ramAttr.addAttribute(.font, value: NSFont.menuFont(ofSize: 13),
                             range: NSRange(location: 0, length: ramAttr.length))
        ramItem.attributedTitle = ramAttr

        // Swap item
        swapItem.title = "Swap: \(snap.swapUsedMB)MB used"

        // Process count
        processItem.title = "Processes: \(snap.processCount) total"
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

        if FileManager.default.isExecutableFile(atPath: pickerPath) {
            // ProcessPicker needs a JSON file from macmon; run macmon to generate it,
            // then launch the picker
            let task = Process()
            task.executableURL = URL(fileURLWithPath: "/bin/bash")
            task.arguments = ["-c", """
                export MACMON_HOME="\(macmonHome)"
                if [ -x "\(macmonBin)" ]; then
                    "\(macmonBin)" picker
                else
                    open "\(pickerPath)"
                fi
            """]
            task.environment = ProcessInfo.processInfo.environment
            try? task.run()
        } else if FileManager.default.isExecutableFile(atPath: macmonBin) {
            // Fall back to CLI picker command
            let task = Process()
            task.executableURL = URL(fileURLWithPath: macmonBin)
            task.arguments = ["picker"]
            var env = ProcessInfo.processInfo.environment
            env["MACMON_HOME"] = macmonHome
            task.environment = env
            try? task.run()
        } else {
            showAlert(
                title: "ProcessPicker Not Found",
                message: "Could not find ProcessPicker at:\n\(pickerPath)\n\nMake sure macmon is installed."
            )
        }
    }

    @objc func exportJSON() {
        runMacmonExport(format: "json")
    }

    @objc func exportCSV() {
        runMacmonExport(format: "csv")
    }

    private func runMacmonExport(format: String) {
        let macmonBin = findMacmonCLI()
        guard FileManager.default.isExecutableFile(atPath: macmonBin) else {
            showAlert(
                title: "macmon Not Found",
                message: "Could not find macmon CLI.\nExpected at: \(macmonBin)"
            )
            return
        }

        // Use NSSavePanel to let the user choose where to save
        let panel = NSSavePanel()
        panel.title = "Export macmon Snapshot"
        panel.nameFieldStringValue = "macmon-snapshot.\(format)"
        if format == "json" {
            panel.allowedContentTypes = [.json]
        }
        panel.canCreateDirectories = true

        let response = panel.runModal()
        guard response == .OK, let url = panel.url else { return }

        let task = Process()
        task.executableURL = URL(fileURLWithPath: macmonBin)
        task.arguments = ["export", format]
        var env = ProcessInfo.processInfo.environment
        env["MACMON_HOME"] = macmonHome
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
            showAlert(
                title: "Export Failed",
                message: "Could not export snapshot: \(error.localizedDescription)"
            )
        }
    }

    @objc func openStatus() {
        let macmonBin = findMacmonCLI()
        // Run macmon status in Terminal.app
        let script: String
        if FileManager.default.isExecutableFile(atPath: macmonBin) {
            script = """
                tell application "Terminal"
                    activate
                    do script "MACMON_HOME='\(macmonHome)' '\(macmonBin)' status"
                end tell
            """
        } else {
            script = """
                tell application "Terminal"
                    activate
                    do script "echo 'macmon CLI not found at \(macmonBin)'"
                end tell
            """
        }

        let appleScript = NSAppleScript(source: script)
        var errorDict: NSDictionary?
        appleScript?.executeAndReturnError(&errorDict)
        if let err = errorDict {
            // If AppleScript fails (e.g. no Terminal access), fall back to direct process
            fputs("Warning: Could not open Terminal via AppleScript: \(err)\n", stderr)
            let task = Process()
            task.executableURL = URL(fileURLWithPath: macmonBin)
            task.arguments = ["status"]
            var env = ProcessInfo.processInfo.environment
            env["MACMON_HOME"] = macmonHome
            task.environment = env
            try? task.run()
        }
    }

    @objc func quitApp() {
        refreshTimer?.invalidate()
        refreshTimer = nil
        NSApp.terminate(nil)
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
        let alert = NSAlert()
        alert.messageText = title
        alert.informativeText = message
        alert.alertStyle = .warning
        alert.addButton(withTitle: "OK")
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

let app = NSApplication.shared
app.setActivationPolicy(.accessory) // Menu bar only, no dock icon
let delegate = StatusBarAppDelegate()
app.delegate = delegate
app.run()
