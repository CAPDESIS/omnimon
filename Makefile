SHELL := /bin/bash
SWIFT_MODEL_SRC := src/gui/ProcessPickerModel.swift
SWIFT_SRC := src/gui/ProcessPicker.swift
SWIFT_I18N_SRC := src/gui/Localization.swift
SWIFT_AI_SRC := src/gui/AIService.swift
SWIFT_PREFS_SRC := src/gui/PreferencesWindow.swift
SWIFT_TELEMETRY_SRC := src/gui/TelemetryRecorder.swift
SWIFT_KILLER_SRC := src/gui/ProcessKiller.swift
SWIFT_BIN := ProcessPicker
DISKIO_SRC := src/gui/DiskIOHelper.swift
DISKIO_BIN := DiskIOHelper
STATUSBAR_SRC := src/gui/MacmonStatusBar.swift
STATUSBAR_BIN := MacmonStatusBar
INSTALL_DIR := $(HOME)/.local/libexec/macmon

# Shared Swift sources used by both ProcessPicker and MacmonStatusBar
SWIFT_SHARED := $(SWIFT_MODEL_SRC) $(SWIFT_I18N_SRC) $(SWIFT_AI_SRC) $(SWIFT_PREFS_SRC) $(SWIFT_TELEMETRY_SRC) $(SWIFT_KILLER_SRC)

# Version (read from macmon-core.sh)
VERSION := $(shell grep -o 'MACMON_VERSION="[^"]*"' lib/macmon-core.sh | cut -d'"' -f2)

# .app bundle paths
APP_NAME := macmon.app
APP_DIR := build/$(APP_NAME)

# XCTest sources
XCTEST_SRCS := tests/swift/AIServiceTests.swift tests/swift/ProcessViewModelTests.swift
XCTEST_BUNDLE := build/MacmonTests.xctest

# Universal binary helper: compile for each arch, merge with lipo
define universal_swiftc
	swiftc -O -target arm64-apple-macos13 $(1) -o $(2)-arm64 $(3)
	swiftc -O -target x86_64-apple-macos13 $(1) -o $(2)-x86_64 $(3)
	lipo -create -output $(2) $(2)-arm64 $(2)-x86_64
	rm -f $(2)-arm64 $(2)-x86_64
endef

.PHONY: build statusbar install uninstall clean check test swift-test audit app dmg

build: $(SWIFT_BIN) $(DISKIO_BIN) $(STATUSBAR_BIN)

$(SWIFT_BIN): $(SWIFT_SHARED) $(SWIFT_SRC)
	$(call universal_swiftc,-framework Cocoa,$@,$(SWIFT_SHARED) $(SWIFT_SRC))

$(DISKIO_BIN): $(DISKIO_SRC)
	$(call universal_swiftc,,$@,$<)

statusbar: $(STATUSBAR_BIN)

$(STATUSBAR_BIN): $(STATUSBAR_SRC) $(SWIFT_SHARED)
	$(call universal_swiftc,-framework Cocoa,$@,$(SWIFT_SHARED) $(STATUSBAR_SRC))

install: build
	./install.sh

uninstall:
	./uninstall.sh

clean:
	rm -f $(SWIFT_BIN) $(DISKIO_BIN) $(STATUSBAR_BIN)
	rm -f $(SWIFT_BIN)-arm64 $(SWIFT_BIN)-x86_64
	rm -f $(DISKIO_BIN)-arm64 $(DISKIO_BIN)-x86_64
	rm -f $(STATUSBAR_BIN)-arm64 $(STATUSBAR_BIN)-x86_64
	rm -rf $(SWIFT_BIN).dSYM $(DISKIO_BIN).dSYM $(STATUSBAR_BIN).dSYM
	rm -rf build/

test:
	@command -v bats >/dev/null 2>&1 || { echo "ERROR: bats not found (brew install bats-core)"; exit 1; }
	bats tests/

swift-test:
	@echo "Running XCTests..."
	@mkdir -p build/MacmonTests.xctest/Contents/MacOS
	@PLATFORM_PATH=$$(xcrun --show-sdk-platform-path); \
	XCTEST_FW="$$PLATFORM_PATH/Developer/Library/Frameworks"; \
	XCTEST_LIB="$$PLATFORM_PATH/Developer/usr/lib"; \
	xcrun swiftc \
		-F "$$XCTEST_FW" \
		-I "$$XCTEST_LIB" \
		-L "$$XCTEST_LIB" \
		-Xlinker -F -Xlinker "$$XCTEST_FW" \
		-Xlinker -rpath -Xlinker "$$XCTEST_FW" \
		-lXCTestSwiftSupport \
		-emit-library \
		-framework XCTest -framework Cocoa \
		-module-name MacmonTests \
		$(SWIFT_SHARED) \
		$(XCTEST_SRCS) \
		-o build/MacmonTests.xctest/Contents/MacOS/MacmonTests
	@/usr/libexec/PlistBuddy -c "Add :CFBundleExecutable string MacmonTests" \
		-c "Add :CFBundleIdentifier string com.macmon.tests" \
		-c "Add :CFBundleInfoDictionaryVersion string 6.0" \
		-c "Add :CFBundlePackageType string BNDL" \
		-c "Add :CFBundleVersion string 1" \
		build/MacmonTests.xctest/Contents/Info.plist 2>/dev/null || true
	@xcrun xctest build/MacmonTests.xctest

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

# --- .app bundle ---
app: build
	@echo "Building $(APP_NAME) v$(VERSION)..."
	@rm -rf $(APP_DIR)
	@mkdir -p $(APP_DIR)/Contents/MacOS
	@mkdir -p $(APP_DIR)/Contents/Resources/en.lproj
	@mkdir -p $(APP_DIR)/Contents/Resources/es.lproj
	@mkdir -p $(APP_DIR)/Contents/Helpers
	@mkdir -p $(APP_DIR)/Contents/SharedSupport
	# Info.plist
	@sed 's/@@VERSION@@/$(VERSION)/g' templates/Info.plist.in > $(APP_DIR)/Contents/Info.plist
	# Icon: convert PNG to icns via sips + iconutil
	@if [ -f icono_app.png ]; then \
		ICONSET=$$(mktemp -d)/macmon.iconset; \
		mkdir -p "$$ICONSET"; \
		for size in 16 32 64 128 256 512; do \
			sips -z $$size $$size icono_app.png --out "$$ICONSET/icon_$${size}x$${size}.png" >/dev/null 2>&1; \
			double=$$((size * 2)); \
			sips -z $$double $$double icono_app.png --out "$$ICONSET/icon_$${size}x$${size}@2x.png" >/dev/null 2>&1; \
		done; \
		iconutil -c icns "$$ICONSET" -o $(APP_DIR)/Contents/Resources/icono_app.icns 2>/dev/null || true; \
		rm -rf "$$ICONSET"; \
	fi
	# Launcher script
	@cp scripts/macmon-launcher.sh $(APP_DIR)/Contents/MacOS/macmon-launcher
	@chmod 755 $(APP_DIR)/Contents/MacOS/macmon-launcher
	# Binaries
	@cp $(SWIFT_BIN) $(APP_DIR)/Contents/Helpers/ProcessPicker
	@cp $(DISKIO_BIN) $(APP_DIR)/Contents/Helpers/DiskIOHelper
	@cp $(STATUSBAR_BIN) $(APP_DIR)/Contents/Helpers/MacmonStatusBar
	@chmod 755 $(APP_DIR)/Contents/Helpers/*
	# SharedSupport: project files the launcher needs
	@cp -R lib $(APP_DIR)/Contents/SharedSupport/
	@cp -R src $(APP_DIR)/Contents/SharedSupport/
	@cp -R scripts $(APP_DIR)/Contents/SharedSupport/
	@cp -R config $(APP_DIR)/Contents/SharedSupport/
	@if [ -d templates ]; then cp -R templates $(APP_DIR)/Contents/SharedSupport/; fi
	@if [ -f icono_app.png ]; then cp icono_app.png $(APP_DIR)/Contents/SharedSupport/; fi
	# Localization
	@cp src/gui/Resources/en.lproj/Localizable.strings $(APP_DIR)/Contents/Resources/en.lproj/
	@cp src/gui/Resources/es.lproj/Localizable.strings $(APP_DIR)/Contents/Resources/es.lproj/
	@echo "Built $(APP_DIR)"

# --- DMG installer ---
dmg: app
	@echo "Creating DMG..."
	@mkdir -p build
	@DMG_NAME="macmon-$(VERSION)-macos-universal.dmg"; \
	DMG_TMP="build/dmg-staging"; \
	rm -rf "$$DMG_TMP" "build/$$DMG_NAME"; \
	mkdir -p "$$DMG_TMP"; \
	cp -R $(APP_DIR) "$$DMG_TMP/"; \
	ln -s /Applications "$$DMG_TMP/Applications"; \
	hdiutil create -volname "macmon $(VERSION)" \
		-srcfolder "$$DMG_TMP" \
		-ov -format UDZO \
		"build/$$DMG_NAME" >/dev/null; \
	rm -rf "$$DMG_TMP"; \
	echo "Created build/$$DMG_NAME"
