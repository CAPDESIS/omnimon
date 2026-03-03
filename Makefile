SHELL := /bin/bash
SWIFT_MODEL_SRC := src/gui/ProcessPickerModel.swift
SWIFT_SRC := src/gui/ProcessPicker.swift
SWIFT_BIN := ProcessPicker
DISKIO_SRC := src/gui/DiskIOHelper.swift
DISKIO_BIN := DiskIOHelper
STATUSBAR_SRC := src/gui/MacmonStatusBar.swift
STATUSBAR_BIN := MacmonStatusBar
INSTALL_DIR := $(HOME)/.local/libexec/macmon

.PHONY: build statusbar install uninstall clean check test

build: $(SWIFT_BIN) $(DISKIO_BIN) $(STATUSBAR_BIN)

$(SWIFT_BIN): $(SWIFT_MODEL_SRC) $(SWIFT_SRC)
	swiftc -O -framework Cocoa -o $@ $(SWIFT_MODEL_SRC) $(SWIFT_SRC)

$(DISKIO_BIN): $(DISKIO_SRC)
	swiftc -O -o $@ $<

statusbar: $(STATUSBAR_BIN)

$(STATUSBAR_BIN): $(STATUSBAR_SRC)
	swiftc -O -framework Cocoa -o $@ $<

install: build
	./install.sh

uninstall:
	./uninstall.sh

clean:
	rm -f $(SWIFT_BIN) $(DISKIO_BIN) $(STATUSBAR_BIN)
	rm -rf $(SWIFT_BIN).dSYM $(DISKIO_BIN).dSYM $(STATUSBAR_BIN).dSYM

test:
	@command -v bats >/dev/null 2>&1 || { echo "ERROR: bats not found (brew install bats-core)"; exit 1; }
	bats tests/

check:
	@echo "Checking dependencies..."
	@command -v jq >/dev/null 2>&1 || { echo "ERROR: jq not found (brew install jq)"; exit 1; }
	@command -v swiftc >/dev/null 2>&1 || { echo "ERROR: swiftc not found (install Xcode CLI tools)"; exit 1; }
	@echo "All dependencies available"
	@echo ""
	@echo "Checking shell scripts..."
	@bash -n lib/macmon-core.sh && echo "  lib/macmon-core.sh: OK"
	@bash -n lib/macmon-config.sh && echo "  lib/macmon-config.sh: OK"
	@bash -n src/daemon/macmond.sh && echo "  src/daemon/macmond.sh: OK"
	@bash -n src/cli/macmon.sh && echo "  src/cli/macmon.sh: OK"
	@bash -n scripts/chrome-tabs.sh && echo "  scripts/chrome-tabs.sh: OK"
	@bash -n scripts/graceful-quit.sh && echo "  scripts/graceful-quit.sh: OK"
	@echo ""
	@echo "Checking Swift compilation..."
	@$(MAKE) build && echo "  Swift binaries: OK"
	@$(MAKE) clean
	@echo ""
	@echo "All checks passed"
