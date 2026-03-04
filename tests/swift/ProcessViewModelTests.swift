import XCTest

// MARK: - JSON Parsing Tests

class ProcessEntryParsingTests: XCTestCase {

    func testParseValidProcessDataJSON() throws {
        let json = """
        {
            "processes": [
                {
                    "pid": 123,
                    "name": "Safari",
                    "ramMB": 512.5,
                    "cpuPct": 3.2,
                    "uptime": "2h30m",
                    "uptimeSeconds": 9000,
                    "cwd": "/Users/test",
                    "tty": "??",
                    "idle": false,
                    "detail": "WebContent",
                    "group": "Browsers",
                    "isSystem": false,
                    "state": "running",
                    "diskReadMB": 100.5,
                    "diskWriteMB": 50.2
                }
            ],
            "system": {
                "freePercent": 45,
                "swapUsedMB": 128,
                "totalProcesses": 300,
                "idleCount": 50,
                "monitoredCount": 20,
                "physMemGB": 16.0
            }
        }
        """
        let data = json.data(using: .utf8)!
        let processData = try JSONDecoder().decode(ProcessData.self, from: data)

        XCTAssertEqual(processData.processes.count, 1)
        let entry = processData.processes[0]
        XCTAssertEqual(entry.pid, 123)
        XCTAssertEqual(entry.name, "Safari")
        XCTAssertEqual(entry.ramMB, 512.5, accuracy: 0.01)
        XCTAssertEqual(entry.cpuPct, 3.2, accuracy: 0.01)
        XCTAssertEqual(entry.uptime, "2h30m")
        XCTAssertEqual(entry.uptimeSeconds, 9000)
        XCTAssertEqual(entry.cwd, "/Users/test")
        XCTAssertEqual(entry.tty, "??")
        XCTAssertFalse(entry.idle)
        XCTAssertEqual(entry.detail, "WebContent")
        XCTAssertEqual(entry.group, "Browsers")
        XCTAssertFalse(entry.isSystem)
        XCTAssertEqual(entry.state, "running")
        XCTAssertEqual(entry.diskReadMB, 100.5, accuracy: 0.01)
        XCTAssertEqual(entry.diskWriteMB, 50.2, accuracy: 0.01)

        XCTAssertEqual(processData.system.freePercent, 45)
        XCTAssertEqual(processData.system.swapUsedMB, 128)
        XCTAssertEqual(processData.system.physMemGB, 16.0, accuracy: 0.01)
    }

    func testParseHandlesMissingOptionalDiskFields() throws {
        let json = """
        {
            "processes": [
                {
                    "pid": 456,
                    "name": "Finder",
                    "ramMB": 200.0,
                    "cpuPct": 1.0,
                    "uptime": "1h",
                    "uptimeSeconds": 3600,
                    "cwd": "/",
                    "tty": "??",
                    "idle": true,
                    "detail": "",
                    "group": "System",
                    "isSystem": true,
                    "state": "idle"
                }
            ],
            "system": {
                "freePercent": 60,
                "swapUsedMB": 0,
                "totalProcesses": 100,
                "idleCount": 30,
                "monitoredCount": 10,
                "physMemGB": 8.0
            }
        }
        """
        let data = json.data(using: .utf8)!
        let processData = try JSONDecoder().decode(ProcessData.self, from: data)

        let entry = processData.processes[0]
        XCTAssertEqual(entry.diskReadMB, 0.0, accuracy: 0.01)
        XCTAssertEqual(entry.diskWriteMB, 0.0, accuracy: 0.01)
        XCTAssertEqual(entry.pid, 456)
        XCTAssertEqual(entry.name, "Finder")
        XCTAssertTrue(entry.idle)
        XCTAssertTrue(entry.isSystem)
    }
}

// MARK: - ViewModel Filter & Sort Tests

class ProcessViewModelFilterSortTests: XCTestCase {

    private func makeViewModel() -> ProcessViewModel {
        let vm = ProcessViewModel()
        vm.allProcesses = [
            ProcessEntry(pid: 1, name: "Safari", ramMB: 500.0, cpuPct: 10.0, group: "Browsers"),
            ProcessEntry(pid: 2, name: "Chrome", ramMB: 1200.0, cpuPct: 25.0, group: "Browsers"),
            ProcessEntry(pid: 3, name: "Terminal", ramMB: 80.0, cpuPct: 2.0, group: "Utilities"),
            ProcessEntry(pid: 4, name: "Xcode", ramMB: 3000.0, cpuPct: 40.0, group: "Development"),
            ProcessEntry(pid: 5, name: "Finder", ramMB: 150.0, cpuPct: 1.0, group: "System", isSystem: true),
        ]
        vm.applyFilterAndSort()
        return vm
    }

    func testFilterBySearchTextNameMatch() {
        let vm = makeViewModel()
        vm.searchText = "safari"
        vm.applyFilterAndSort()

        XCTAssertEqual(vm.filteredIndices.count, 1)
        XCTAssertEqual(vm.allProcesses[vm.filteredIndices[0]].name, "Safari")
    }

    func testFilterBySearchTextPIDMatch() {
        let vm = makeViewModel()
        vm.searchText = "3"
        vm.applyFilterAndSort()

        // PID 3 is "Terminal"
        let names = vm.filteredIndices.map { vm.allProcesses[$0].name }
        XCTAssertTrue(names.contains("Terminal"))
    }

    func testFilterBySearchTextNoMatch() {
        let vm = makeViewModel()
        vm.searchText = "zzzznotfound"
        vm.applyFilterAndSort()

        XCTAssertEqual(vm.filteredIndices.count, 0)
        XCTAssertEqual(vm.displayRows.count, 0)
    }

    func testSortByRamMBDescendingPutsHighestFirst() {
        let vm = makeViewModel()
        vm.sortColumn = .ramMB
        vm.sortOrder = .descending
        vm.applyFilterAndSort()

        let ramValues = vm.filteredIndices.map { vm.allProcesses[$0].ramMB }
        for i in 0..<(ramValues.count - 1) {
            XCTAssertGreaterThanOrEqual(ramValues[i], ramValues[i + 1],
                "RAM values should be in descending order: \(ramValues)")
        }
        XCTAssertEqual(vm.allProcesses[vm.filteredIndices[0]].name, "Xcode")
    }

    func testSortByNameAscendingIsAlphabetical() {
        let vm = makeViewModel()
        vm.sortColumn = .name
        vm.sortOrder = .ascending
        vm.applyFilterAndSort()

        let names = vm.filteredIndices.map { vm.allProcesses[$0].name }
        let expected = ["Chrome", "Finder", "Safari", "Terminal", "Xcode"]
        XCTAssertEqual(names, expected, "Names should be alphabetically sorted ascending")
    }

    func testSortOrderToggleReversesResults() {
        let vm = makeViewModel()
        vm.sortColumn = .ramMB
        vm.sortOrder = .ascending
        vm.applyFilterAndSort()
        let ascendingOrder = vm.filteredIndices

        vm.sortOrder = .descending
        vm.applyFilterAndSort()
        let descendingOrder = vm.filteredIndices

        XCTAssertEqual(ascendingOrder, descendingOrder.reversed(),
            "Toggling sort order should reverse the result order")
    }
}

// MARK: - ViewModel Grouping Tests

class ProcessViewModelGroupingTests: XCTestCase {

    private func makeGroupedViewModel() -> ProcessViewModel {
        let vm = ProcessViewModel()
        vm.allProcesses = [
            ProcessEntry(pid: 1, name: "Safari", ramMB: 500.0, cpuPct: 10.0, group: "Browsers"),
            ProcessEntry(pid: 2, name: "Chrome", ramMB: 1200.0, cpuPct: 25.0, group: "Browsers"),
            ProcessEntry(pid: 3, name: "Terminal", ramMB: 80.0, cpuPct: 2.0, group: "Utilities"),
        ]
        vm.groupingEnabled = true
        vm.sortColumn = .name
        vm.sortOrder = .ascending
        vm.applyFilterAndSort()
        return vm
    }

    func testGroupingCreatesCorrectGroupHeaders() {
        let vm = makeGroupedViewModel()

        // Collect group header names
        var headerNames: [String] = []
        for row in vm.displayRows {
            if case .groupHeader(let name, _, _, _) = row {
                headerNames.append(name)
            }
        }

        XCTAssertTrue(headerNames.contains("Browsers"), "Should have Browsers group")
        XCTAssertTrue(headerNames.contains("Utilities"), "Should have Utilities group")
        XCTAssertEqual(headerNames.count, 2)

        // Check that Browsers group header has count 2
        for row in vm.displayRows {
            if case .groupHeader(let name, let count, _, _) = row, name == "Browsers" {
                XCTAssertEqual(count, 2, "Browsers group should have 2 processes")
            }
        }
    }

    func testCollapsingGroupHidesProcessRows() {
        let vm = makeGroupedViewModel()

        let totalRowsBefore = vm.displayRows.count
        // Browsers group: 1 header + 2 processes; Utilities: 1 header + 1 process = 5 total
        XCTAssertEqual(totalRowsBefore, 5, "Should have 5 display rows before collapse")

        // Collapse the Browsers group
        vm.toggleGroup("Browsers")

        let totalRowsAfter = vm.displayRows.count
        // After collapsing Browsers: 1 header (collapsed) + 0 processes + 1 header + 1 process = 3
        XCTAssertEqual(totalRowsAfter, 3, "Should have 3 display rows after collapsing Browsers")

        // Verify the Browsers header is now collapsed
        for row in vm.displayRows {
            if case .groupHeader(let name, _, _, let collapsed) = row, name == "Browsers" {
                XCTAssertTrue(collapsed, "Browsers group should be collapsed")
            }
        }

        // Verify no process rows for the Browsers group indices
        let browserProcessIndices = vm.displayRows.compactMap { row -> Int? in
            if case .process(let idx) = row { return idx }
            return nil
        }
        for idx in browserProcessIndices {
            XCTAssertNotEqual(vm.allProcesses[idx].group, "Browsers",
                "No process rows from Browsers group should be visible when collapsed")
        }
    }
}

// MARK: - ViewModel Selection Tests

class ProcessViewModelSelectionTests: XCTestCase {

    func testSelectAllSkipsSystemProcesses() {
        let vm = ProcessViewModel()
        vm.allProcesses = [
            ProcessEntry(pid: 1, name: "Safari", ramMB: 500.0, cpuPct: 10.0, isSystem: false),
            ProcessEntry(pid: 2, name: "kernel_task", ramMB: 2000.0, cpuPct: 5.0, isSystem: true),
            ProcessEntry(pid: 3, name: "Chrome", ramMB: 800.0, cpuPct: 15.0, isSystem: false),
        ]
        vm.applyFilterAndSort()

        // Simulate selectAll: mark all non-system processes in filteredIndices
        for i in vm.filteredIndices {
            if !vm.allProcesses[i].isSystem {
                vm.allProcesses[i].selected = true
            }
        }

        XCTAssertTrue(vm.allProcesses[0].selected || vm.allProcesses[2].selected,
            "At least one non-system process should be selected")

        // kernel_task (pid 2, index 1) should NOT be selected
        let kernelIndex = vm.allProcesses.firstIndex(where: { $0.pid == 2 })!
        XCTAssertFalse(vm.allProcesses[kernelIndex].selected,
            "System process kernel_task should not be selected by selectAll")

        // All non-system should be selected
        for i in vm.filteredIndices {
            if !vm.allProcesses[i].isSystem {
                XCTAssertTrue(vm.allProcesses[i].selected,
                    "\(vm.allProcesses[i].name) should be selected")
            }
        }
    }

    func testSelectedRAMComputation() {
        let vm = ProcessViewModel()
        vm.allProcesses = [
            ProcessEntry(pid: 1, name: "App1", ramMB: 100.0, cpuPct: 1.0),
            ProcessEntry(pid: 2, name: "App2", ramMB: 250.5, cpuPct: 2.0),
            ProcessEntry(pid: 3, name: "App3", ramMB: 400.0, cpuPct: 3.0),
        ]
        vm.applyFilterAndSort()

        // Select App1 and App3
        vm.allProcesses[0].selected = true
        vm.allProcesses[2].selected = true

        XCTAssertEqual(vm.selectedRAM, 500.0, accuracy: 0.01,
            "Selected RAM should be 100.0 + 400.0 = 500.0")
        XCTAssertEqual(vm.selectedCount, 2)
        XCTAssertEqual(Set(vm.selectedPIDs), Set([1, 3]))
    }
}

// MARK: - Config Assistant Tests

class ConfigQuickSettingsTests: XCTestCase {

    func testParseQuickSettingsFromYAML() {
        let yaml = """
        thresholds:
          ram_free_percent: 33
          swap_used_mb: 1536
          process_ram_min_kb: 204800
          idle_cpu_percent: 2.5

        intervals:
          check: 45
          idle_check: 500
          cooldown: 200
          kill_grace: 5

        collect:
          disk_io: false
        """

        let settings = ConfigQuickSettings.parse(from: yaml)
        XCTAssertEqual(settings.ramFreePercent, 33)
        XCTAssertEqual(settings.swapUsedMB, 1536)
        XCTAssertEqual(settings.processMinRAMKB, 204800)
        XCTAssertEqual(settings.idleCPUPercent, 2.5, accuracy: 0.001)
        XCTAssertEqual(settings.checkIntervalSec, 45)
        XCTAssertEqual(settings.idleCheckSec, 500)
        XCTAssertEqual(settings.cooldownSec, 200)
        XCTAssertEqual(settings.killGraceSec, 5)
        XCTAssertFalse(settings.collectDiskIO)
    }

    func testRenderQuickSettingsYAMLContainsExpectedKeys() {
        var settings = ConfigQuickSettings()
        settings.ramFreePercent = 40
        settings.swapUsedMB = 1024
        settings.processMinRAMKB = 51200
        settings.idleCPUPercent = 1.25
        settings.checkIntervalSec = 30
        settings.idleCheckSec = 300
        settings.cooldownSec = 120
        settings.killGraceSec = 4
        settings.collectDiskIO = true

        let yaml = settings.renderYAML()
        XCTAssertTrue(yaml.contains("ram_free_percent: 40"))
        XCTAssertTrue(yaml.contains("swap_used_mb: 1024"))
        XCTAssertTrue(yaml.contains("process_ram_min_kb: 51200"))
        XCTAssertTrue(yaml.contains("idle_cpu_percent: 1.25"))
        XCTAssertTrue(yaml.contains("collect:"))
        XCTAssertTrue(yaml.contains("disk_io: true"))
    }
}
