# OmniMon Homebrew Tap (macOS)

Guide for distributing OmniMon to macOS users (Apple Silicon and x86_64) via **Homebrew Casks**.

## Setup

The Homebrew tap repo is [`chochy2001/homebrew-omnimon`](https://github.com/chochy2001/homebrew-omnimon). It contains:

- `Formula/omnimon.rb` — CLI formula (legacy v3 bash CLI)
- `Casks/omnimon.rb` — Desktop app Cask (v4 Tauri app)

## Releasing a New Version

When publishing a new release:

1. Update `version` and `sha256` in `Casks/omnimon.rb`:
   ```bash
   shasum -a 256 macmon_X.Y.Z_aarch64.dmg
   ```
2. Push to `homebrew-omnimon` main branch.

## User Install

```bash
brew tap chochy2001/omnimon
brew install --cask omnimon   # Desktop app
brew install omnimon           # CLI
```

Homebrew auto-updates Casks on `brew upgrade`.
