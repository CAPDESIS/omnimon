# OmniMon Architecture

This document describes the high-level architecture of OmniMon, focusing on the communication and security of the system.

## Secure IPC Bridge

OmniMon uses a robust Inter-Process Communication (IPC) bridge to safely communicate between the frontend and the system's native backend.

### AppleScript RCE Mitigation
To securely execute AppleScript for tasks like browser tab introspection without the risk of Remote Code Execution (RCE) via argument injection, OmniMon avoids string interpolation of user-provided data into the scripts.
Instead, AppleScripts utilize the `on run argv` handler. Arguments are passed strictly as positional parameters using the `-e` flag with `osascript`:
```rust
let mut cmd = Command::new("osascript");
cmd.arg("-e");
cmd.arg(script); // The static script containing 'on run argv'
cmd.arg(user_provided_arg1); // Passed securely as positional args
cmd.arg(user_provided_arg2);
```

### CDP WebSocket Validation
For the Chrome Debugging Protocol (CDP), the system ensures that WebSocket endpoints are not susceptible to path traversal. Any `tab_id` sent from the frontend is strictly validated before being used to construct connection URLs. The system actively rejects characters like `/`, `\`, `?`, and `#`, guaranteeing that connections are only made to valid, authorized debugging endpoints.
