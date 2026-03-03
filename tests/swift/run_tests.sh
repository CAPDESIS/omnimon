#!/usr/bin/env bash
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$DIR/../.." && pwd)"

XCODE_DEV="$(xcode-select -p)"
PLATFORM_DIR="$XCODE_DEV/Platforms/MacOSX.platform/Developer"
FRAMEWORK_DIR="$PLATFORM_DIR/Library/Frameworks"
USR_LIB_DIR="$PLATFORM_DIR/usr/lib"

BUNDLE_DIR="$DIR/ProcessViewModelTests.xctest"
CONTENTS_DIR="$BUNDLE_DIR/Contents/MacOS"

rm -rf "$BUNDLE_DIR"
mkdir -p "$CONTENTS_DIR"

# Write Info.plist
cat > "$BUNDLE_DIR/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>ProcessViewModelTests</string>
    <key>CFBundleIdentifier</key>
    <string>com.macmon.ProcessViewModelTests</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundlePackageType</key>
    <string>BNDL</string>
    <key>CFBundleVersion</key>
    <string>1</string>
</dict>
</plist>
PLIST

# Compile as a loadable bundle (.xctest)
swiftc \
    -emit-library \
    -o "$CONTENTS_DIR/ProcessViewModelTests" \
    -module-name ProcessViewModelTests \
    -F "$FRAMEWORK_DIR" \
    -I "$USR_LIB_DIR" \
    -L "$USR_LIB_DIR" \
    -framework XCTest \
    -lXCTestSwiftSupport \
    -Xlinker -rpath -Xlinker "$FRAMEWORK_DIR" \
    -Xlinker -rpath -Xlinker "$USR_LIB_DIR" \
    -Xlinker -bundle \
    "$REPO/src/gui/ProcessPickerModel.swift" \
    "$REPO/src/gui/Localization.swift" \
    "$REPO/src/gui/AIService.swift" \
    "$DIR/ProcessViewModelTests.swift" \
    "$DIR/AIServiceTests.swift"

# Run tests using xctest
xcrun xctest "$BUNDLE_DIR"

# Clean up
rm -rf "$BUNDLE_DIR"
