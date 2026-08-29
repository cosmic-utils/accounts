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

# Build the app. One binary is the GUI, the background D-Bus service, and the
# consent prompt — selected by ACCOUNTS_HEADLESS / ACCOUNTS_CONSENT_PROMPT.
build-gui:
    cargo build --release -p accounts_ui

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

# Install the app system-wide (requires sudo). One binary serves both the GUI
# and the D-Bus-activated background service (see crates/ui/data/cosmic-accounts.service).
install-gui: build-gui
    sudo install -Dm0755 target/release/accounts_ui /usr/bin/accounts_ui
    # D-Bus activation: one activation file + wrapper per mode of the shared
    # binary (the wrappers set ACCOUNTS_HEADLESS / ACCOUNTS_CONSENT_PROMPT,
    # which .service files can't).
    sudo install -Dm0755 crates/ui/data/accounts-ui-daemon /usr/bin/accounts-ui-daemon
    sudo install -Dm0755 crates/ui/data/accounts-consent-prompt /usr/bin/accounts-consent-prompt
    sudo install -Dm0644 crates/ui/data/dev.edfloreshz.Accounts.service /usr/share/dbus-1/services/dev.edfloreshz.Accounts.service
    sudo install -Dm0644 crates/ui/data/dev.edfloreshz.Accounts.ConsentPrompt.service /usr/share/dbus-1/services/dev.edfloreshz.Accounts.ConsentPrompt.service
    # Optional systemd user unit (for `just start-daemon`), in the systemd path,
    # not the D-Bus one.
    sudo install -Dm0644 crates/ui/data/cosmic-accounts.service /usr/lib/systemd/user/cosmic-accounts.service
    sudo install -Dm0644 crates/ui/resources/app.desktop /usr/share/applications/dev.edfloreshz.Accounts.desktop
    sudo install -Dm0644 crates/ui/resources/app.metainfo.xml /usr/share/metainfo/dev.edfloreshz.Accounts.metainfo.xml
    sudo install -Dm0644 crates/ui/resources/icons/hicolor/scalable/apps/icon.svg /usr/share/icons/hicolor/scalable/apps/dev.edfloreshz.Accounts.svg
    sudo install -Dm0644 crates/ui/data/dev.edfloreshz.Accounts.policy /usr/share/polkit-1/actions/dev.edfloreshz.Accounts.policy

# Install provider configurations (requires sudo). ProviderRegistry reads
# $XDG_DATA_DIRS/accounts/providers, i.e. /usr/share/accounts/providers.
install-configs:
    sudo install -Dm0644 crates/ui/data/providers/google.toml /usr/share/accounts/providers/google.toml
    sudo install -Dm0644 crates/ui/data/providers/microsoft.toml /usr/share/accounts/providers/microsoft.toml
    @echo "Remember to set OAuth2 client credentials in /usr/share/accounts/providers/"

# Install everything (requires sudo)
install: build install-gui install-configs

# Uninstall system files (requires sudo)
uninstall:
    sudo rm -f /usr/bin/accounts_ui
    sudo rm -f /usr/bin/accounts-ui-daemon
    sudo rm -f /usr/bin/accounts-consent-prompt
    sudo rm -f /usr/bin/accounts-consent-helper
    sudo rm -f /usr/share/dbus-1/services/dev.edfloreshz.Accounts.service
    sudo rm -f /usr/share/dbus-1/services/dev.edfloreshz.Accounts.ConsentPrompt.service
    sudo rm -f /usr/lib/systemd/user/cosmic-accounts.service
    sudo rm -f /usr/share/dbus-1/services/cosmic-accounts.service
    sudo rm -f /usr/share/applications/dev.edfloreshz.Accounts.desktop
    sudo rm -f /usr/share/metainfo/dev.edfloreshz.Accounts.metainfo.xml
    sudo rm -f /usr/share/icons/hicolor/scalable/apps/dev.edfloreshz.Accounts.svg
    sudo rm -f /usr/share/polkit-1/actions/dev.edfloreshz.Accounts.policy
    sudo rm -rf /usr/share/accounts /etc/accounts

# Start the background service (user session)
start-daemon:
    systemctl --user enable cosmic-accounts.service
    systemctl --user start cosmic-accounts.service

# Stop the background service (user session)
stop-daemon:
    systemctl --user stop cosmic-accounts.service
    systemctl --user disable cosmic-accounts.service

# Check background service status
status:
    systemctl --user status cosmic-accounts.service

# View background service logs
logs:
    journalctl --user -u cosmic-accounts.service -f

# Run CLI tool with list command
cli-list:
    cargo run --example cli -- list

# Run CLI tool with help
cli-help:
    cargo run --example cli -- --help

# Run the background D-Bus service in the foreground with debug logging,
# without opening a window. Run from the repo root so it picks up
# crates/ui/data/providers/*.toml.
run-daemon:
    ACCOUNTS_HEADLESS=1 RUST_LOG=debug cargo run -p accounts_ui

# Run the GUI in the foreground with debug logging.
run-ui:
    RUST_LOG=debug cargo run -p accounts_ui

# Run the reference ProviderHandler (fake device-code flow) on the session bus.
# Point a provider manifest's [handler] at
# dev.edfloreshz.Accounts.ProviderHandler.Example /dev/edfloreshz/Accounts/ProviderHandler.
run-example-handler:
    RUST_LOG=debug cargo run -p accounts_example_handler

# Run the headless service + the GUI together for a full local test session.
# Ctrl-C stops the whole stack. Requires a running D-Bus session bus and a
# secret-service provider (gnome-keyring/kwallet) for credential storage;
# OAuth flows additionally need real client credentials in
# crates/ui/data/providers/*.toml.
run-stack:
    #!/usr/bin/env bash
    set -euo pipefail
    trap 'kill 0' EXIT
    echo "Starting background service..."
    ACCOUNTS_HEADLESS=1 RUST_LOG=debug cargo run -p accounts_ui &
    sleep 1
    echo "Starting UI..."
    RUST_LOG=debug cargo run -p accounts_ui

# List providers currently advertised by a running daemon over D-Bus
# (requires busctl; useful to confirm capability advertisement without the UI).
list-providers:
    busctl --user call dev.edfloreshz.Accounts /dev/edfloreshz/Accounts/Account \
        dev.edfloreshz.Accounts.Account ListProviders

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

# Development: full workspace build check
workspace-check:
    cargo check --workspace

# Package for distribution (creates release builds and archives)
package: clean build
    mkdir -p dist
    cp target/release/accounts_ui dist/ || echo "Build failed, skipping"
    cp -r crates/ui/data dist/
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
    @echo "   Ubuntu/Debian: sudo apt install libdbus-1-dev libsecret-1-dev"
    @echo "   Fedora: sudo dnf install dbus-devel libsecret-devel"
    @echo "   Arch: sudo pacman -S dbus libsecret"
    @echo "3. Install development tools:"
    @echo "   cargo install cargo-watch cargo-audit cargo-outdated"
    @echo "4. Run 'just check' to verify everything works"
