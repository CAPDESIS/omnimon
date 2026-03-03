# Implementation Tracker

Status of macmon features and planned work.

## Completed

- Dynamic process monitoring with configurable thresholds per process
- Internationalization support (English and Spanish)
- PID file handling with lock protection
- Config hot-reload on file changes and SIGUSR1
- Profile system with presets (developer, creator, gaming)
- Optional AI-assisted process analysis via external providers
- Protected process blocklist with Apple code signature verification

## Planned

- Dedicated UI for editing profile YAML files
- Integration tests for the full daemon cycle
- Dry-run mode for previewing optimizations
- Signed profile export/import
- Release checklist automation

## Quality Checks

```bash
make check                    # dependencies + syntax + compilation
make test                     # BATS test suite
bash tests/swift/run_tests.sh # Swift XCTests
make audit                    # shellcheck + CVE review
```
