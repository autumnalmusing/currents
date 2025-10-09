# Justfile for currents project
# Install just: https://github.com/casey/just

# List all available recipes
default:
    @just --list

# Build the project in debug mode
build:
    cargo build

# Build the project in release mode
build-release:
    cargo build --release

# Run all tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run a specific test
test-one TEST:
    cargo test {{TEST}} -- --nocapture

# Run the daemon in foreground mode
run:
    cargo run -- --foreground

# Display the 5-day forecast
forecast:
    cargo run -- --forecast

# Check API stats
api-stats:
    cargo run -- --api-stats

# Run clippy linter
lint:
    cargo clippy -- -D warnings

# Format code
fmt:
    cargo fmt

# Check formatting without modifying files
fmt-check:
    cargo fmt -- --check

# Clean build artifacts
clean:
    cargo clean

# Install the binary locally
install:
    cargo install --path .

# Build the Nix flake
nix-build:
    nix build

# Check the Nix flake
nix-check:
    nix flake check

# Verify Nix flake and module
nix-verify:
    nix flake check

# Update Cargo dependencies
update-deps:
    cargo update

# Generate and open documentation
docs:
    cargo doc --open

# Run all checks (tests, lint, format)
check: test lint fmt-check
    @echo "All checks passed!"

# Watch for changes and run tests
watch-test:
    cargo watch -x test

# Watch for changes and run the daemon
watch-run:
    cargo watch -x 'run -- --foreground'

# Build and install systemd service
install-service: build-release
    ./install-service.sh

# View daemon logs
logs:
    sudo journalctl -u currents@$USER -f

# Restart the daemon
restart:
    sudo systemctl restart currents@$USER

# Check daemon status
status:
    sudo systemctl status currents@$USER

