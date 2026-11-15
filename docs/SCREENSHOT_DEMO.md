# Native screenshot demo

The normal app is unchanged. In a debug build only, `OMNIMON_DEMO=1` selects
an isolated Tauri host before the normal setup runs. It renders the production
Svelte UI using `demo-data.json` through the normal IPC response validators.

```sh
cd v4/apps/desktop
OMNIMON_DEMO=1 bun run tauri dev
```

For a standalone debug bundle, use `bun run tauri build --debug --bundles app`,
then execute its binary with `OMNIMON_DEMO=1`. Building alone does not enable
the demo. Never open a bundle without that variable for screenshot work.

The demo host starts no watcher, packet capture, automation, Zombie Killer
engine, AI provider, browser integration or process control. Only a fixed
read-only command allowlist responds. Other app commands fail closed. The
store plugin uses the separate `com.omnimon.screenshot-demo` identifier and
resets its preferences to English/dark on startup. Its webview is incognito. It never reads the normal
app's preferences. Theme and locale can be changed through the actual UI.
The window closes normally; it does not hide in the tray.

All process names and metrics are fictional constants. Executable paths are
null, browser tabs are empty and network addresses are documentation ranges.
DPI is false. The Zombie Killer candidate is fictional; its action buttons
cannot terminate any process. This is capture infrastructure, not a product
feature or evidence of real monitoring results.

The runtime and fixture module are excluded with `cfg(debug_assertions)` from
release builds. `bash scripts/test-demo-mode.sh` compiles and runs the same
opt-in gate with debug assertions enabled and disabled. Cargo tests cover the
read-only command boundary, and the frontend fixture test exercises the real
IPC validators. Close the demo and verify its PID has exited after capture.

The fixture includes 22 generic processes and eight documentation-range connections.
RAM and network totals derive from the rows; the status CPU is the mean per process,
matching StatusBar. Allow network polling to complete before capturing process names.
Capture without the cursor and move focus away from controls before saving images.
The isolated host emits the same synthetic metrics every three seconds to fill
the normal chart history. No host measurements are taken.
