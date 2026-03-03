# Contributing

Thanks for your interest in macmon! This is a personal project, but contributions are welcome.

## How to Contribute

1. Fork the repository and create a feature branch
2. Make your changes
3. Run `make check` to verify dependencies and compilation
4. Run `make test` to run the test suite
5. Submit a pull request

## Development Setup

```bash
brew install jq bats-core    # dependencies
xcode-select --install       # Swift compiler

make check    # verify everything works
make test     # run tests
```

## Guidelines

- All code and commit messages should be in English
- Follow conventional commit style: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`
- Shell scripts must pass `bash -n` syntax check
- Swift code must compile with `swiftc -O -framework Cocoa` (no Xcode project required)
- Add tests for new functionality when possible

## Reporting Issues

Open an issue on GitHub with:
- macOS version
- Steps to reproduce
- Expected vs actual behavior
- Relevant log output (`macmon log`)
