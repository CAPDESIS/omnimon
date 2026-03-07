# Backend Remediation Plan (Rust Core & Tauri IPC)

**Date**: 2026-03-07
**Scope**: Only `.rs` files. No frontend, config, or CI changes.

---

## SEV1-01: `.expect()` crash on Tauri init failure

- **File**: `v4/apps/desktop/src-tauri/src/lib.rs` line 563
- **Problem**: `.expect("error while running tauri application")` panics if Tauri
  fails to initialize. No recovery, no error message to user.
- **Fix**: Replace `.expect()` with `match` or `if let Err(e)` that logs the error
  via `eprintln!` and exits with a non-zero code instead of panicking.
  In Tauri v2, `.run()` returns `Result<(), tauri::Error>`. We handle it gracefully.

## SEV2-03: TAB_REFRESH_IN_PROGRESS permanent deadlock on panic

- **File**: `v4/apps/desktop/src-tauri/src/lib.rs` lines 174-200
- **Problem**: If the AppleScript/CDP tab refresh work panics between setting the
  flag to `true` and the `store(false)`, the AtomicBool stays `true` forever.
  All future tab refreshes are blocked permanently.
- **Fix**: Wrap the expensive work in `std::panic::catch_unwind()`. In the catch
  branch, reset `TAB_REFRESH_IN_PROGRESS` to `false` and return stale cache.
  This ensures the flag is always reset regardless of panics.

## SEV2-04: Watcher thread dies silently on panic

- **File**: `v4/crates/core/src/watcher.rs` lines 131-198
- **Problem**: The infinite `loop` inside the watcher thread has no panic boundary.
  If any operation panics (collect_state, network sampling, rules eval), the
  thread terminates and metrics freeze forever with no indication.
- **Fix**: Wrap the loop body in `std::panic::catch_unwind()`. On panic, log to
  stderr and `continue` the loop. This way a transient panic in one tick
  doesn't kill the entire monitoring pipeline. The outer loop survives.

## SEV2-05: TOCTOU in killer.rs identity_matches + inverted logic

- **File**: `v4/crates/core/src/killer.rs` lines 258-273
- **Problem**: After sending SIGTERM and waiting 120ms, the code checks if the
  process is still alive AND if identity matches. The logic at line 262-266:
  ```
  if !identity_matches(...) || crate::os_native::kill_process_force(...).is_err()
  ```
  If identity doesn't match, it means the original process died and a new one
  took the PID. This should be treated as SUCCESS (original is dead), but the
  current code falls through to `false` (failure).
- **Fix**: Separate the identity check from the force-kill path. If identity
  doesn't match after graceful kill, the original process is dead -> return
  success. Only attempt force kill if identity still matches (same process
  survived SIGTERM).
