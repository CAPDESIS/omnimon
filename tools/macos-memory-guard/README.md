# OmniMon Memory Guard (macOS)

Deterministic LaunchAgent that watches RAM pressure and **never** auto-kills
Warp, Chrome, IDEs, agent CLIs, or Apple's `fseventsd`.

This is the open-source OmniMon sibling of the in-app Zombie Killer:

| | Zombie Killer (app) | Memory Guard (LaunchAgent) |
|---|---|---|
| Where | Desktop UI | `launchd`, no GUI in the hot path |
| What | Sustained CPU/RAM, user click to kill | Leftover orphans (idle, `ppid=1`, ≥1h) |
| False positives | `never_kill` + immutable OS names | Pure classifier (`prove` table) |

## Install

```bash
bash tools/macos-memory-guard/install-memory-guard.sh
omnimon-memory-guard prove    # must print OK
omnimon-memory-guard status
```

State lives under `~/.local/state/omnimon-memory-guard/` (outside Documents, TCC-safe).

## Commands

```bash
omnimon-memory-guard prove              # classifier contract, no live processes
omnimon-memory-guard selftest           # parsers + prove
omnimon-memory-guard status
omnimon-memory-guard offenders
omnimon-memory-guard reap               # leftovers only
omnimon-memory-guard sessions           # Warp/agent sessions older than 30 days
omnimon-memory-guard review-sessions    # only yes/no dialog
omnimon-memory-guard relieve-fsevents   # stop idle git fsmonitor; never kills fseventsd
```

To drop Apple `fseventsd` heap (launchd relaunches it):

```bash
sudo killall fseventsd
```

## Uninstall

```bash
bash tools/macos-memory-guard/uninstall-memory-guard.sh
# optional: --purge-state
```

## License

MIT, same as OmniMon.
