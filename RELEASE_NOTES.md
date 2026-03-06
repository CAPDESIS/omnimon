# OmniMon v4 Release 🚀

We are excited to announce the production release of OmniMon v4! This release finalizes the transition to a modern, zero-leak Rust architecture, complete with advanced security and automation capabilities.

## 🌟 Key Highlights

### 🛡️ MITRE ATT&CK & Dynamic eBPF Detections
This release includes major enhancements to our threat detection capabilities. The new **AiConfigBridge** allows the dynamic creation of rules (e.g., GeoIP matching), which the Rust backend processes in real-time. For Linux users, the eBPF (`aya`) collector now achieves full feature parity with macOS/Windows by resolving destination IPs directly from kernel maps.

### ☁️ CrabNebula Cloud Integration
OmniMon is now fully integrated with the CrabNebula backend. You can securely authenticate using your OS's native Keyring (`omnimon auth login`) and sync encrypted security reports using the new CLI commands. The Tauri Auto-Updater is also wired directly to CrabNebula's CDN to ensure you receive future updates securely and automatically.

### ⚡ LTO & Hyper-Optimized Binaries
Thanks to Link-Time Optimization (LTO) and strict build profiles (`codegen-units=1`, `opt-level=s`), the compiled binaries and payload sizes have been significantly reduced, preserving memory specifically for our native drivers (`libpcap` / `WinDivert`) and enforcing a true "Zero-Leak" runtime.

---

## 🔒 Security Posture & NIST Compliance
Every build runs an automated vulnerability scan using `Grype` matching the NIST SP 800-53 Framework guidelines.
**Current Security Posture:** Scan data not available locally.

---

## 📝 Recent Commits since last release

- feat(ci): universal macOS binary + RPM for Fedora/RHEL (4beb1d3)
- fix(ci): auto-rename release assets to user-friendly platform names (f934f7f)
- fix(ci): publish releases directly instead of creating drafts (aaa8146)

## 📥 Artifacts
All platform artifacts (`.dmg`, `.exe`, `.deb`) have been generated and signed cryptographically by our CI/CD pipeline. Use `omnimon doctor` after installation to verify native driver health.
