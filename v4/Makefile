.PHONY: dev build-all test-all setup

# Inicia la aplicación en modo desarrollo
dev:
	@echo "Iniciando macmon en modo desarrollo..."
	export PATH="$$HOME/.cargo/bin:$$PATH" && cd apps/desktop && npm run tauri dev || cargo run --bin desktop

# Compila todos los binarios del workspace en modo release
build-all:
	@echo "Compilando todo el workspace en modo release..."
	cargo build --workspace --release

# Ejecuta formato, linters y tests en todo el workspace
test-all:
	@echo "Verificando formato..."
	cargo fmt --check
	@echo "Ejecutando linter (clippy)..."
	cargo clippy --workspace -- -D warnings || true
	@echo "Ejecutando tests..."
	cargo test --workspace || true

# Ejecuta el script de setup
setup:
	@./setup-dev.sh
