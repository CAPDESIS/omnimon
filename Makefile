SHELL := /bin/bash
SWIFT_SRC := src/gui/ProcessPicker.swift
SWIFT_BIN := ProcessPicker
INSTALL_DIR := $(HOME)/.local/libexec/macmon

.PHONY: build install uninstall clean check

build: $(SWIFT_BIN)

$(SWIFT_BIN): $(SWIFT_SRC)
	swiftc -O -framework Cocoa -o $@ $<

install: build
	./install.sh

uninstall:
	./uninstall.sh

clean:
	rm -f $(SWIFT_BIN)
	rm -rf $(SWIFT_BIN).dSYM

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
	@$(MAKE) build && echo "  ProcessPicker.swift: OK"
	@$(MAKE) clean
	@echo ""
	@echo "All checks passed"
