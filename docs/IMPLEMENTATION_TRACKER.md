# Implementation Tracker

This file tracks completed milestones and next steps for macmon.

## Completed

1. Dynamic process monitoring from YAML custom_processes.
2. i18n scaffolding for UI and CLI messages.
3. Reliability hardening for PID handling and config fallback.
4. Profile system with hot swap from CLI and AppKit.
5. AI analysis integration with Keychain backed API keys.
6. Human in the loop optimization flow with explicit user approval.
7. Immutable process blocklist and Apple system process safeguards.
8. LLM hallucination mitigation using regex fallback PID extraction.
9. PID sanitization against immutable blocklist and live process validation.

## In Progress

1. Expanded UI level diagnostics for AI response quality scoring.
2. Optional dry run mode for optimization preview in menu bar.

## Next Steps

1. Add dedicated UI for viewing and editing profile YAML files.
2. Add integration tests that mock provider responses end to end.
3. Add local telemetry opt in for false positive and false negative suggestions.
4. Add a signed profile export and import format.
5. Add release checklist automation for docs and screenshots.

## Validation Checklist

1. `make check`
2. `make test`
3. `bash tests/swift/run_tests.sh`
4. `make audit`
5. `make verify-authors`
