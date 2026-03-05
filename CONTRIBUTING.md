# Contributing to OmniMon

Thanks for your interest in contributing to OmniMon! As an open source project, we rely on the community to improve, stabilize, and expand the tool across all platforms.

## Development Environment

Setting up the cross-platform environment (Rust, Tauri, Svelte) is straightforward with our orchestration scripts.

1. **Clone the repository:**
   ```bash
   git clone https://github.com/chochy2001/omnimon.git
   cd omnimon
   ```

2. **Run the setup script:**
   * macOS/Linux: `./v4/setup-dev.sh`
   * Windows: `.\v4\setup-dev.ps1`

   This script checks and/or installs Node.js, Rust, Cargo, and native OS dependencies like WebView2 (Windows) or libwebkit2gtk (Linux).

3. **Start development mode:**
   ```bash
   cd v4
   make dev
   ```
   This compiles the Rust backend and launches the Tauri interface with Vite/Svelte hot-reloading.

## Cross-Platform Requirements

OmniMon v4 is designed to run natively on **macOS, Windows, and Linux**. Any new feature or module (e.g. browser tab tracking, native OS interactions) **must** be supported on all three platforms, or degrade gracefully if the OS API doesn't support it.

* Before proposing a new feature, ensure the code compiles and passes tests on all three environments.
* Use Rust's `#[cfg(target_os = "...")]` typing for OS-specific implementations.
* **CI/CD will automatically validate** your changes on Ubuntu, macOS, and Windows runners. If your Pull Request breaks the build on any platform, it cannot be merged.

## Workflow and Pull Requests

1. Fork the project and work on a descriptive branch, e.g. `feat/my-new-feature` or `fix/bug-fix`.
2. Implement your changes (avoid mixing frontend logic in native core crates without proper IPC justification).
3. **Critical checkpoint:** Verify your code meets standards:
   ```bash
   cd v4
   make test-all
   ```
   This runs `cargo fmt`, `cargo clippy --workspace -- -D warnings`, and `cargo test`. **Your PR will not be accepted if GitHub CI fails or detects warnings.**
4. Open a Pull Request against the `main` branch clearly describing what problem your code solves and how to test it.

## Commit Convention (Conventional Commits)

We require Conventional Commits to maintain a clean history and generate reliable changelogs.
* `feat:` New features (e.g. `feat(ai): add Claude 3.5 support`).
* `fix:` Bug fixes (e.g. `fix(core): prevent hang when reading nonexistent process`).
* `docs:` Documentation-only changes (README, SECURITY, CONTRIBUTING, `/docs`).
* `chore:` Infrastructure maintenance, dependencies, or release processes.
* `refactor:` Code refactoring without altering observable behavior.
* `test:` Adding or fixing tests.

We're excited to review your contributions!
