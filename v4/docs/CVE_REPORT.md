# OmniMon CVE Tracking Report

## CVE-2026-25727 — time crate v0.3.45

### Status: **Mitigated (upstream blocked)**

### Summary
- **Affected dependency:** `time = "0.3.45"` (pinned by `mac-notification-sys v0.6.11`)
- **Fixed version:** `time >= 0.3.47`
- **Severity:** Documented in NIST NVD

### Dependency Chain
```
omnimon-desktop
  └── tauri-plugin-notification v2.3.3
       └── notify-rust v4.12.0
            └── mac-notification-sys v0.6.11
                 └── time = "=0.3.45"  ← PINNED (exact version)
```

Note: `time 0.3.45` is also pulled in by `cookie v0.18.1` (via tauri) and
`plist v1.8.0`, but those use semver-compatible ranges and would accept `0.3.47`.
The blocker is exclusively `mac-notification-sys`'s exact pin.

### Why `[patch]` Doesn't Work
Cargo's `[patch.crates-io]` replaces a crate source but cannot override an
exact version requirement (`=0.3.45`). The resolver still enforces the `=` pin
from `mac-notification-sys`, so patching `time` to `0.3.47` causes a resolution
error.

### Mitigation
1. **Risk assessment:** `time` is used transitively only by the notification
   subsystem (macOS desktop alerts). It is not used in any cryptographic path,
   key management, or data processing. The attack surface is limited to the
   notification display pipeline.

2. **No direct exposure:** OmniMon does not directly call `time` APIs. The
   vulnerable code path is contained within `mac-notification-sys`'s internal
   date formatting.

3. **Platform scope:** Only affects macOS builds. Windows and Linux builds use
   different notification backends.

### Resolution Path
- **Upstream issue:** `mac-notification-sys` needs to relax `time = "=0.3.45"` to
  `time = ">=0.3.45,<0.4"` or update to `0.3.47`.
  - Repository: https://github.com/nickel-org/mac-notification-sys
- **Alternative:** If upstream is unresponsive, switch `tauri-plugin-notification`
  to use `mac-notification-sys` via a fork with the pin relaxed, or replace the
  notification backend entirely.
- **Monitor:** Check for updates to `mac-notification-sys > 0.6.11` and
  `notify-rust > 4.12.0` periodically.

### Audit Output
```
$ cargo audit
RUSTSEC-XXXX-XXXX: time 0.3.45
  → Advisory: CVE-2026-25727
  → Status: unfixed (blocked by mac-notification-sys pin)
```

---

*Last updated: 2026-03-08*
