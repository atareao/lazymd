default: run

# Build the project (debug)
build:
    cargo build

# Build the project (release)
build-release:
    cargo build --release

# Run the TUI
run:
    cargo run

# Run all tests
test:
    cargo test

# Lint with clippy
clippy:
    cargo clippy -- -D warnings

# Format code (edition 2024 style)
fmt:
    cargo fmt

# Check compilation without building (fast)
check:
    cargo check

# Fix common warnings automatically
fix:
    cargo fix --allow-dirty

# Full pre-commit check: fmt + clippy + test
precommit: fmt clippy test

# Clean build artifacts
clean:
    cargo clean

# Generate docs
doc:
    cargo doc --no-deps

# Open docs in browser
doc-open:
    cargo doc --no-deps --open

# Update CodeGraph index
codegraph:
    codegraph init -i

# Show CodeGraph status
codegraph-status:
    codegraph status