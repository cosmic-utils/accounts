# Accounts for COSMIC - Build and Installation Commands

name := 'accounts'
ui-name := 'accounts-ui'
appid := 'dev.edfloreshz.Accounts'
rootdir := ''
prefix := '/usr'
base-dir := absolute_path(clean(rootdir / prefix))
etc-dst := clean(rootdir) / 'etc' / 'accounts' / 'providers'
bin-dst := base-dir / 'bin'
dbus-dst := clean(rootdir / prefix) / 'share' / 'dbus-1' / 'services'

desktop := appid + '.desktop'
desktop-src := ui-name / 'resources' / desktop
desktop-dst := clean(rootdir / prefix) / 'share' / 'applications' / desktop
appdata := appid + '.metainfo.xml'
appdata-src := ui-name / 'resources' / appdata
appdata-dst := clean(rootdir / prefix) / 'share' / 'appdata' / appdata
icons-src := ui-name / 'resources' / 'icons' / 'hicolor'
icons-dst := clean(rootdir / prefix) / 'share' / 'icons' / 'hicolor'
icon-svg-src := icons-src / 'scalable' / 'apps' / 'icon.svg'
icon-svg-dst := icons-dst / 'scalable' / 'apps' / appid + '.svg'

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

# Build the provider processes
build-providers:
    cargo build --release -p accounts-provider-google -p accounts-provider-microsoft

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
    install -Dm0755 target/release/accounts-daemon -t {{ bin-dst }}
    install -Dm0644 accounts-daemon/data/cosmic-accounts.service -t {{ dbus-dst }}

# Install GUI system-wide (requires sudo)
install-gui: build-gui
    install -Dm0755 target/release/accounts-ui -t {{ bin-dst }}
    install -Dm0644 accounts-ui/resources/app.desktop {{desktop-dst}}
    install -Dm0644 accounts-ui/resources/app.metainfo.xml {{appdata-dst}}
    install -Dm0644 {{icon-svg-src}} {{icon-svg-dst}}

# Install provider configurations (requires sudo)
install-configs:
    install -Dm0644 accounts-daemon/data/providers/*.toml -t {{ etc-dst }}
    @echo "Remember to update OAuth2 credentials in /etc/accounts/providers/"

# Install everything (requires sudo)
install: build install-daemon install-gui install-configs

# Uninstall system files (requires sudo)
uninstall:
    sudo rm -f /usr/bin/accounts-daemon
    sudo rm -f /usr/bin/accounts-ui
    sudo rm -f /usr/share/dbus-1/services/accounts.service
    sudo rm -rf /etc/accounts

# Start the daemon service (user session)
start-daemon:
    systemctl --user enable accounts.service
    systemctl --user start accounts.service

# Stop the daemon service (user session)
stop-daemon:
    systemctl --user stop accounts.service
    systemctl --user disable accounts.service

# Check daemon status
status:
    systemctl --user status accounts.service

# View daemon logs
logs:
    journalctl --user -u accounts.service -f

# Run CLI tool with list command
cli-list:
    cargo run --example cli -- list

# Run CLI tool with help
cli-help:
    cargo run --example cli -- --help

# Run the Google provider process in the foreground with debug logging.
# Registers dev.edfloreshz.Accounts.Provider.Google on the session bus.
run-google:
    RUST_LOG=debug cargo run -p accounts-provider-google

# Run the Microsoft provider process in the foreground with debug logging.
# Registers dev.edfloreshz.Accounts.Provider.Microsoft on the session bus.
run-microsoft:
    RUST_LOG=debug cargo run -p accounts-provider-microsoft

# Run both provider processes in the foreground (Ctrl-C stops both).
# Uses the manifests in accounts-daemon/data/providers/, which ProviderRegistry
# reads as a dev-mode fallback without needing anything installed.
run-providers:
    #!/usr/bin/env bash
    set -euo pipefail
    trap 'kill 0' EXIT
    RUST_LOG=debug cargo run -p accounts-provider-google &
    RUST_LOG=debug cargo run -p accounts-provider-microsoft &
    wait

# Run the daemon in the foreground with debug logging.
# Run from the repo root so it picks up accounts-daemon/data/providers/*.toml.
run-daemon:
    RUST_LOG=debug cargo run -p accounts-daemon

# Run the GUI in the foreground with debug logging.
run-ui:
    RUST_LOG=debug cargo run -p accounts-ui

# Run providers + daemon + UI together for a full local test session.
# Ctrl-C stops the whole stack. Requires a running D-Bus session bus and
# secret-service provider (gnome-keyring/kwallet) for the daemon's credential
# storage; OAuth flows additionally need real client credentials in
# accounts-daemon/data/providers/*.toml.
run-stack:
    #!/usr/bin/env bash
    set -euo pipefail
    trap 'kill 0' EXIT
    echo "Starting providers..."
    RUST_LOG=debug cargo run -p accounts-provider-google &
    RUST_LOG=debug cargo run -p accounts-provider-microsoft &
    sleep 1
    echo "Starting daemon..."
    RUST_LOG=debug cargo run -p accounts-daemon &
    sleep 1
    echo "Starting UI..."
    RUST_LOG=debug cargo run -p accounts-ui

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
    @echo "   Ubuntu/Debian: sudo apt install libdbus-1-dev libsecret-1-dev"
    @echo "   Fedora: sudo dnf install dbus-devel libsecret-devel"
    @echo "   Arch: sudo pacman -S dbus libsecret"
    @echo "3. Install development tools:"
    @echo "   cargo install cargo-watch cargo-audit cargo-outdated"
    @echo "4. Run 'just check' to verify everything works"
