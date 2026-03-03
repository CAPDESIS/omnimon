# Versioning

macmon follows [Semantic Versioning](https://semver.org/) (SemVer).

## Version Format

```
MAJOR.MINOR.PATCH
```

- **MAJOR**: Breaking changes to CLI interface, config format, or plist structure
- **MINOR**: New features, new CLI commands, new config options (backward compatible)
- **PATCH**: Bug fixes, performance improvements, documentation updates

## Current Version

The version is defined in `lib/macmon-core.sh`:
```bash
MACMON_VERSION="1.2.0"
```

All components read this single source of truth.

## Release Process

### 1. Update the version

Edit `lib/macmon-core.sh` and set `MACMON_VERSION`:
```bash
MACMON_VERSION="X.Y.Z"
```

### 2. Update CHANGELOG.md

Add a new section at the top of `CHANGELOG.md`:
```markdown
## X.Y.Z (YYYY-MM-DD)

### Features
- Description of new features

### Fixes
- Description of bug fixes

### Security
- Description of security improvements
```

### 3. Commit and tag

```bash
git add lib/macmon-core.sh CHANGELOG.md
git commit -m "Release vX.Y.Z"
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin main --tags
```

### 4. GitHub Release

Pushing a tag triggers the release workflow (`.github/workflows/release.yml`), which:
1. Runs all tests (BATS + Swift compilation)
2. Builds all binaries
3. Creates a `.tar.gz` archive
4. Publishes a GitHub Release with auto-generated release notes

### 5. Update Homebrew formula (optional)

After the release, update `brew/macmon.rb`:
1. Update the `url` to point to the new tag archive
2. Update the `sha256` with `shasum -a 256 macmon-X.Y.Z-macos-arm64.tar.gz`

## Version History

| Version | Date | Highlights |
|---------|------|------------|
| 1.2.0 | 2026-03-03 | Menu bar, MVVM refactor, XCTests, Homebrew, release workflow |
| 1.1.0 | 2026-03-03 | Orphan daemons, disk I/O, export, BATS tests, CI |
| 1.0.0 | 2026-03-03 | Initial release: daemon, picker, CLI |

## Checking Your Version

```bash
macmon version    # prints: macmon v1.2.0
```

## Upgrading

### From source (git)
```bash
cd /path/to/macmon
git pull origin main
./install.sh
```

### From Homebrew (future)
```bash
brew upgrade macmon
```

The installer handles all upgrades: recompiles Swift binaries, updates scripts, preserves user config, and restarts the daemon.
