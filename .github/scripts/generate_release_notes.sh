#!/bin/bash
# generate_release_notes.sh
# Generates release notes incorporating recent commits, NIST report status, and DevSecOps highlights.

OUTPUT_FILE="RELEASE_NOTES.md"
NIST_REPORT="nist-compliance-report.html"

# Ensure we're in a git repo
if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "Error: Not a git repository."
    exit 1
fi

echo "Generating Release Notes..."

# Get the latest tag, or default to the beginning if none exists
LATEST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || git rev-list --max-parents=0 HEAD)
COMMITS=$(git log ${LATEST_TAG}..HEAD --pretty=format:"- %s (%h)")

cat <<EOF > "$OUTPUT_FILE"
# OmniMon v4 Release 🚀

We are excited to announce the production release of OmniMon v4! This release finalizes the transition to a modern, zero-leak Rust architecture, complete with advanced security and automation capabilities.

## 🌟 Key Highlights

### 🛡️ MITRE ATT&CK & Dynamic eBPF Detections
This release includes major enhancements to our threat detection capabilities. The new **AiConfigBridge** allows the dynamic creation of rules (e.g., GeoIP matching), which the Rust backend processes in real-time. For Linux users, the eBPF (\`aya\`) collector now achieves full feature parity with macOS/Windows by resolving destination IPs directly from kernel maps.

### ☁️ CrabNebula Cloud Integration
OmniMon is now fully integrated with the CrabNebula backend. You can securely authenticate using your OS's native Keyring (\`omnimon auth login\`) and sync encrypted security reports using the new CLI commands. The Tauri Auto-Updater is also wired directly to CrabNebula's CDN to ensure you receive future updates securely and automatically.

### ⚡ LTO & Hyper-Optimized Binaries
Thanks to Link-Time Optimization (LTO) and strict build profiles (\`codegen-units=1\`, \`opt-level=s\`), the compiled binaries and payload sizes have been significantly reduced, preserving memory specifically for our native drivers (\`libpcap\` / \`WinDivert\`) and enforcing a true "Zero-Leak" runtime.

---

## 🔒 Security Posture & NIST Compliance
Every build runs an automated vulnerability scan using \`Grype\` matching the NIST SP 800-53 Framework guidelines.
EOF

# Parse NIST report if it exists to get the general posture
if [ -f "$NIST_REPORT" ]; then
    if grep -q "Critical Risk" "$NIST_REPORT"; then
        echo "**Current Security Posture:** 🚨 Critical Risk Detected (Please review the generated NIST report)." >> "$OUTPUT_FILE"
    elif grep -q "Elevated Risk" "$NIST_REPORT"; then
        echo "**Current Security Posture:** ⚠️ Elevated Risk Detected." >> "$OUTPUT_FILE"
    else
        echo "**Current Security Posture:** ✅ Healthy (No Critical/High vulnerabilities detected)." >> "$OUTPUT_FILE"
    fi
else
    echo "**Current Security Posture:** Scan data not available locally." >> "$OUTPUT_FILE"
fi

cat <<EOF >> "$OUTPUT_FILE"

---

## 📝 Recent Commits since last release

$COMMITS

## 📥 Artifacts
All platform artifacts (\`.dmg\`, \`.exe\`, \`.deb\`) have been generated and signed cryptographically by our CI/CD pipeline. Use \`omnimon doctor\` after installation to verify native driver health.
EOF

echo "Release notes successfully generated in $OUTPUT_FILE"
