# COSMIC Accounts - Build and Installation Commands

# Default recipe - show available commands
default:
    @just --list

# Build all components
build:
    cargo build --release

# Build only the library
build-lib:
    cargo build --release --lib

# Build the daemon
build-daemon:
    cargo build --release -p cosmic-accounts-daemon

# Build the GUI (may fail if dependencies aren't available)
build-gui:
    cargo build --release -p cosmic-accounts-gui

# Run all tests
test:
    cargo test --lib
    cargo test --test integration_test

# Run clippy linting
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Format code
format:
    cargo fmt

# Check formatting
check-format:
    cargo fmt --check

# Run all checks (test, lint, format)
check: test lint check-format

# Clean build artifacts
clean:
    cargo clean

# Install daemon system-wide (requires sudo)
install-daemon: build-daemon
    sudo cp target/release/cosmic-accounts-daemon /usr/bin/
    sudo cp data/cosmic-accounts.service /usr/share/dbus-1/services/

# Install GUI system-wide (requires sudo)
install-gui: build-gui
    sudo cp target/release/cosmic-accounts-gui /usr/bin/

# Install provider configurations (requires sudo)
install-configs:
    sudo mkdir -p /etc/cosmic-accounts/providers
    sudo cp data/providers/*.toml /etc/cosmic-accounts/providers/
    @echo "Remember to update OAuth2 credentials in /etc/cosmic-accounts/providers/"

# Install everything (requires sudo)
install: build install-daemon install-gui install-configs

# Uninstall system files (requires sudo)
uninstall:
    sudo rm -f /usr/bin/cosmic-accounts-daemon
    sudo rm -f /usr/bin/cosmic-accounts-gui
    sudo rm -f /usr/share/dbus-1/services/cosmic-accounts.service
    sudo rm -rf /etc/cosmic-accounts

# Start the daemon service (user session)
start-daemon:
    systemctl --user enable cosmic-accounts.service
    systemctl --user start cosmic-accounts.service

# Stop the daemon service (user session)
stop-daemon:
    systemctl --user stop cosmic-accounts.service
    systemctl --user disable cosmic-accounts.service

# Check daemon status
status:
    systemctl --user status cosmic-accounts.service

# View daemon logs
logs:
    journalctl --user -u cosmic-accounts.service -f

# Run CLI tool with list command
cli-list:
    cargo run --example cli -- list

# Run CLI tool with help
cli-help:
    cargo run --example cli -- --help

# Development: run daemon in foreground with debug logging
dev-daemon:
    RUST_LOG=debug cargo run --example daemon

# Development: watch for changes and run tests
dev-watch:
    cargo watch -x "test --lib" -x "test --test integration_test"

# Generate documentation
docs:
    cargo doc --no-deps --open

# Check dependencies for security advisories
audit:
    cargo audit

# Update dependencies
update:
    cargo update

# Check for outdated dependencies
outdated:
    cargo outdated

# Benchmark (if any benchmarks exist)
bench:
    cargo bench

# Example: add a Google account (interactive)
example-add-google:
    @echo "Starting Google account addition..."
    cargo run --example cli -- add Google

# Example: show all accounts
example-show-accounts:
    cargo run --example cli -- list

# Development: quick check without running tests
quick-check:
    cargo check --workspace --exclude cosmic-accounts-gui

# Development: full workspace build check
workspace-check:
    cargo check --workspace

# Package for distribution (creates release builds and archives)
package: clean build
    mkdir -p dist
    cp target/release/cosmic-accounts-daemon dist/
    cp target/release/cosmic-accounts-gui dist/ || echo "GUI build failed, skipping"
    cp -r data dist/
    cp README.md dist/
    cp LICENSE* dist/ || echo "No license files found"
    tar czf dist/cosmic-accounts-$(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "cosmic-accounts") | .version').tar.gz -C dist .

# Development: run daemon and CLI in separate terminals
dev-split:
    @echo "Run 'just dev-daemon' in one terminal and 'just cli-list' in another"

# Help for setting up development environment
dev-setup:
    @echo "Development setup:"
    @echo "1. Install Rust toolchain: https://rustup.rs/"
    @echo "2. Install system dependencies:"
    @echo "   Ubuntu/Debian: sudo apt install libdbus-1-dev libsecret-1-dev"
    @echo "   Fedora: sudo dnf install dbus-devel libsecret-devel"
    @echo "   Arch: sudo pacman -S dbus libsecret"
    @echo "3. Install development tools:"
    @echo "   cargo install cargo-watch cargo-audit cargo-outdated"
    @echo "4. Run 'just check' to verify everything works"
