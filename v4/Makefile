.PHONY: dev build-all test-all setup

# Start the app in development mode
dev:
	@echo "Starting OmniMon in development mode..."
	export PATH="$$HOME/.cargo/bin:$$PATH" && cd apps/desktop && npm run tauri dev || cargo run --bin desktop

# Build all workspace binaries in release mode
build-all:
	@echo "Building entire workspace in release mode..."
	cargo build --workspace --release

# Run formatting, linting, and tests across the workspace
test-all:
	@echo "Checking formatting..."
	cargo fmt --check
	@echo "Running linter (clippy)..."
	cargo clippy --workspace -- -D warnings || true
	@echo "Running tests..."
	cargo test --workspace || true

# Run the setup script
setup:
	@./setup-dev.sh
