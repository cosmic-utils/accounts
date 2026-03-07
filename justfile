# Accounts for COSMIC - Build and Installation Commands

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
    cargo build --release -p accounts-daemon

# Build the GUI (may fail if dependencies aren't available)
build-gui:
    cargo build --release -p accounts-ui

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
    sudo cp target/release/accounts-daemon /usr/bin/
    sudo cp accounts-daemon/data/cosmic-accounts.service /usr/lib/systemd/user/

# Install GUI system-wide (requires sudo)
install-gui: build-gui
    sudo cp target/release/accounts-ui /usr/bin/

# Install provider configurations (requires sudo)
install-configs:
    sudo mkdir -p /etc/accounts/providers
    sudo cp accounts-daemon/data/providers/*.toml /etc/accounts/providers/
    @echo "Remember to update OAuth2 credentials in /etc/accounts/providers/"

# Install desktop entry and icons (requires sudo)
install-desktop:
    sudo cp accounts-ui/resources/app.desktop /usr/share/applications/dev.edfloreshz.Accounts.desktop
    sudo update-desktop-database /usr/share/applications/ 2>/dev/null || true

# Install everything (requires sudo)
install: build install-daemon install-gui install-configs install-desktop

# Uninstall system files (requires sudo)
uninstall:
    sudo rm -f /usr/bin/accounts-daemon
    sudo rm -f /usr/bin/accounts-ui
    sudo rm -f /usr/lib/systemd/user/cosmic-accounts.service
    sudo rm -f /usr/share/applications/dev.edfloreshz.Accounts.desktop
    sudo rm -rf /etc/accounts

# Start the daemon and launch the UI
start: start-daemon
    /usr/bin/accounts-ui

# Start the daemon service (user session)
start-daemon:
    systemctl --user daemon-reload
    systemctl --user enable cosmic-accounts
    systemctl --user start cosmic-accounts

# Stop the daemon service (user session)
stop-daemon:
    systemctl --user stop cosmic-accounts
    systemctl --user disable cosmic-accounts

# Check daemon status
status:
    systemctl --user status cosmic-accounts

# View daemon logs
logs:
    journalctl --user -u cosmic-accounts -f

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
    cargo check --workspace --exclude accounts-ui

# Development: full workspace build check
workspace-check:
    cargo check --workspace

# Package for distribution (creates release builds and archives)
package: clean build
    mkdir -p dist
    cp target/release/accounts-daemon dist/
    cp target/release/accounts-ui dist/ || echo "GUI build failed, skipping"
    cp -r data dist/
    cp README.md dist/
    cp LICENSE* dist/ || echo "No license files found"
    tar czf dist/accounts-$(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "accounts") | .version').tar.gz -C dist .

# Development: run daemon and CLI in separate terminals
dev-split:
    @echo "Run 'just dev-daemon' in one terminal and 'just cli-list' in another"

# Help for setting up development environment
dev-setup:
    @echo "Development setup:"
    @echo "1. Install Rust toolchain: https://rustup.rs/"
    @echo "2. Install system dependencies:"
    @echo "   Ubuntu/Debian: sudo apt install libssl-dev pkg-config libdbus-1-dev libsecret-1-dev libxkbcommon-dev libwayland-dev"
    @echo "   Fedora: sudo dnf install dbus-devel libsecret-devel"
    @echo "   Arch: sudo pacman -S dbus libsecret"
    @echo "3. Install development tools:"
    @echo "   cargo install cargo-watch cargo-audit cargo-outdated"
    @echo "4. Run 'just check' to verify everything works"
