# Security Model

This document describes the current security controls in macmon and how they map to common attacker techniques.

## Threat Model

- macmon runs as a user-level app (LaunchAgent), not root.
- The highest-risk path is process termination requests (manual + AI-assisted suggestions).
- LLM responses are always treated as untrusted input.

## Core Controls

- Immutable protected process list in Bash and Swift.
- Apple-signed system binary checks before any kill action.
- PID/name re-validation before SIGTERM and before SIGKILL.
- Human-in-the-loop confirmations in GUI flows.
- API keys stored in macOS Keychain (not plaintext config).
- Strict PID extraction/parsing (no shell interpolation for commands).

## MITRE ATT&CK Alignment (High-Level)

- T1059 (Command and Scripting Interpreter)
  - Mitigation: no direct shell execution from LLM output; validated IDs only.
- T1562 (Impair Defenses)
  - Mitigation: system process kill-blocks + signature verification.
- T1036 (Masquerading)
  - Mitigation: process name checks plus Apple code-sign identity checks.
- T1565 (Data Manipulation via config)
  - Mitigation: guarded config reload flow + safe defaults + protected process floor.

## CVE and Dependency Hygiene

- Primary runtime dependencies: bash, jq, Swift stdlib, macOS frameworks.
- `jq` is required for structured parsing in update and process JSON handling.
- CI should run tests on every change and release tags.
- Any discovered vulnerability in updater/install path is release-blocking.

## Residual Risk

- Chrome renderer-to-tab mapping is best-effort due browser internals.
- AppleScript permissions can reduce metadata quality for tabs.

## Responsible Disclosure

Please report security issues privately before opening a public issue.
