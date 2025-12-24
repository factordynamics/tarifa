# Default recipe to list available commands
default:
    @just --list

# Aliases
alias t := test
alias c := check
alias b := build

# Run the full CI suite
ci: fmt-check clippy test udeps

# Run tests
test:
    cargo nextest run --all-features

# Run tests in watch mode
watch-test:
    cargo watch -x 'nextest run --all-features'

# Check formatting and run clippy
check: fmt-check clippy test

# Run clippy lints
clippy:
    cargo clippy --all-targets --all-features -- -D warnings -D unknown-lints

# Watch clippy
watch-check:
    cargo watch -x 'clippy --all-targets --all-features -- -D warnings'

# Format check
fmt-check:
    cargo +nightly fmt --all -- --check

# Format code
fmt:
    cargo +nightly fmt --all

# Auto-fix formatting and clippy issues
fix:
    cargo +nightly fmt --all
    cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged

# Build release
build:
    cargo build --release

# Generate and open documentation
doc:
    cargo doc --all-features --no-deps --open

# Check for unused dependencies
udeps:
    cargo +nightly udeps --all-features

# Clean build artifacts
clean:
    cargo clean

# ============ Tarifa-specific commands ============

# List available signals
signals category="":
    cargo run --release -- signals {{ if category != "" { "--category " + category } else { "" } }}

# Evaluate a signal
eval signal +symbols:
    cargo run --release -- eval {{ signal }} --symbols {{ symbols }}

# Run backtest
backtest signal start end:
    cargo run --release -- backtest {{ signal }} --start {{ start }} --end {{ end }}

# Show signal scores
score signal +symbols:
    cargo run --release -- score {{ signal }} {{ symbols }}
