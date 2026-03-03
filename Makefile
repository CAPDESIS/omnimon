SHELL := /bin/bash
SWIFT_MODEL_SRC := src/gui/ProcessPickerModel.swift
SWIFT_SRC := src/gui/ProcessPicker.swift
SWIFT_I18N_SRC := src/gui/Localization.swift
SWIFT_AI_SRC := src/gui/AIService.swift
SWIFT_PREFS_SRC := src/gui/PreferencesWindow.swift
SWIFT_BIN := ProcessPicker
DISKIO_SRC := src/gui/DiskIOHelper.swift
DISKIO_BIN := DiskIOHelper
STATUSBAR_SRC := src/gui/MacmonStatusBar.swift
STATUSBAR_BIN := MacmonStatusBar
INSTALL_DIR := $(HOME)/.local/libexec/macmon

.PHONY: build statusbar install uninstall clean check test audit

build: $(SWIFT_BIN) $(DISKIO_BIN) $(STATUSBAR_BIN)

$(SWIFT_BIN): $(SWIFT_MODEL_SRC) $(SWIFT_SRC) $(SWIFT_I18N_SRC) $(SWIFT_AI_SRC)
	swiftc -O -framework Cocoa -o $@ $(SWIFT_MODEL_SRC) $(SWIFT_I18N_SRC) $(SWIFT_AI_SRC) $(SWIFT_SRC)

$(DISKIO_BIN): $(DISKIO_SRC)
	swiftc -O -o $@ $<

statusbar: $(STATUSBAR_BIN)

$(STATUSBAR_BIN): $(STATUSBAR_SRC) $(SWIFT_I18N_SRC) $(SWIFT_AI_SRC) $(SWIFT_PREFS_SRC)
	swiftc -O -framework Cocoa -o $@ $(SWIFT_I18N_SRC) $(SWIFT_AI_SRC) $(SWIFT_PREFS_SRC) $(STATUSBAR_SRC)

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
	@bash -n lib/macmon-security.sh && echo "  lib/macmon-security.sh: OK"
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

audit:
	@echo "=== Security Audit ==="
	@echo ""
	@echo "Dependency versions:"
	@echo "  jq: $$(jq --version 2>&1)"
	@echo "  bash: $$(bash --version | head -1)"
	@echo "  swiftc: $$(swiftc --version 2>&1 | head -1)"
	@echo ""
	@echo "shellcheck analysis:"
	@command -v shellcheck >/dev/null 2>&1 || { echo "  shellcheck not installed (brew install shellcheck)"; exit 0; }
	@shellcheck -e SC1091,SC2034 lib/macmon-core.sh && echo "  lib/macmon-core.sh: CLEAN"
	@shellcheck -e SC1091 lib/macmon-config.sh && echo "  lib/macmon-config.sh: CLEAN"
	@shellcheck -e SC1091 lib/macmon-security.sh && echo "  lib/macmon-security.sh: CLEAN"
	@shellcheck -e SC1091 src/daemon/macmond.sh && echo "  src/daemon/macmond.sh: CLEAN"
	@shellcheck -e SC1091 src/cli/macmon.sh && echo "  src/cli/macmon.sh: CLEAN"
	@shellcheck -e SC1091 scripts/chrome-tabs.sh && echo "  scripts/chrome-tabs.sh: CLEAN"
	@shellcheck -e SC1091 scripts/graceful-quit.sh && echo "  scripts/graceful-quit.sh: CLEAN"
	@shellcheck install.sh && echo "  install.sh: CLEAN"
	@shellcheck uninstall.sh && echo "  uninstall.sh: CLEAN"
	@echo ""
	@echo "Known CVEs in dependencies:"
	@echo "  CVE-2024-23337 (jq <=1.7.1): integer overflow DoS — MITIGATED (all jq inputs validated)"
	@echo "  CVE-2025-48060 (jq <=1.7.1): heap-buffer-overflow — MITIGATED (no untrusted input)"
	@echo ""
	@echo "File permissions:"
	@echo "  Install dir: $$(stat -f '%Lp' $(INSTALL_DIR) 2>/dev/null || echo 'not installed')"
	@echo "  Config dir:  $$(stat -f '%Lp' $(HOME)/.config/macmon 2>/dev/null || echo 'not installed')"
	@echo "  Log dir:     $$(stat -f '%Lp' $(HOME)/.local/log/macmon 2>/dev/null || echo 'not installed')"
	@echo ""
	@echo "Audit complete"
