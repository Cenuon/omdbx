.DEFAULT_GOAL := dev

PNPM ?= pnpm
GPUI_DIR ?= apps/desktop-gpui

.PHONY: help dev dev-backend build build-backend package-macos clean cargo-check-fast cargo-test-fast db db-list db-verify db-down db-reset db-check db-completion

export DB
export DB_VERSION
export DB_BIND_ADDRESS
export DB_PORT
export DB_PASSWORD
export FOLLOW
export CONFIRM

help:
	@printf '%s\n' 'DBX development targets:'
	@printf '%s\n' ''
	@printf '%s\n' 'App:'
	@printf '  %-23s %s\n' 'make' 'Start the dbx-web backend and the GPUI desktop app'
	@printf '  %-23s %s\n' 'make dev' 'Start the dbx-web backend and the GPUI desktop app'
	@printf '  %-23s %s\n' 'make dev-backend' 'Start the dbx-web backend only (127.0.0.1:4225)'
	@printf '  %-23s %s\n' 'make build-backend' 'Build the dbx-web release binary'
	@printf '  %-23s %s\n' 'make build' 'Build the dbx-web backend and the GPUI app'
	@printf '  %-23s %s\n' 'make package-macos' 'Build a macOS .app, .dmg, and zip'
	@printf '  %-23s %s\n' 'make clean' 'Remove local Rust build artifacts and caches'
	@printf '%s\n' ''
	@printf '%s\n' 'Checks:'
	@printf '  %-23s %s\n' 'make cargo-check-fast' 'Run Rust check without default features'
	@printf '  %-23s %s\n' 'make cargo-test-fast' 'Run Rust tests without default features'
	@printf '%s\n' ''
	@printf '%s\n' 'Database test environments:'
	@printf '  %-23s %s\n' 'make db-list' 'List available database versions'
	@printf '  %-23s %s\n' 'make db DB=mysql@8.4' 'Start and print DBX connection fields'
	@printf '  %-23s %s\n' 'make db-verify DB=mysql@8.4' 'Start and run smoke checks'
	@printf '  %-23s %s\n' 'make db-down DB=mysql@8.4' 'Stop an environment'
	@printf '  %-23s %s\n' 'make db-reset DB=mysql@8.4 CONFIRM=1' 'Delete containers and data'
	@printf '  %-23s %s\n' 'make db-check' 'Validate every recipe and Compose file'
	@printf '  %-23s %s\n' 'make db-completion' 'Show Bash/Zsh completion setup'
	@printf '%s\n' ''

build-backend:
	cargo build --release -p dbx-web --no-default-features --features duckdb-sidecar,dynamodb,mq-admin,dbx-core/sqlite-bundled

dev: build-backend
	sh $(GPUI_DIR)/dev.sh

dev-backend: build-backend
	sh $(GPUI_DIR)/run-backend.sh

build: build-backend
	cd $(GPUI_DIR) && cargo build --release

package-macos: build
	sh $(GPUI_DIR)/scripts/package-macos.sh

clean:
	cargo clean
	cd $(GPUI_DIR) && cargo clean

cargo-check-fast:
	cargo check --no-default-features --features sqlite-bundled

cargo-test-fast:
	RUST_MIN_STACK=8388608 cargo nextest run --no-default-features --features sqlite-bundled --no-fail-fast
	RUST_MIN_STACK=8388608 cargo test --doc --no-default-features --features sqlite-bundled

db-list:
	@$(PNPM) db:env -- list

db:
	@$(PNPM) db:env -- start

db-verify:
	@$(PNPM) db:env -- verify

db-down:
	@$(PNPM) db:env -- down

db-reset:
	@$(PNPM) db:env -- reset

db-check:
	@$(PNPM) db:env -- check

db-completion:
	@$(PNPM) db:env -- completion
