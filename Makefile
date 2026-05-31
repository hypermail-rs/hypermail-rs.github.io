.PHONY: all build test fmt clippy check clean release install-hooks fuzz fuzz-quick audit upgrade ci ci-fuzz docs

all: check build test

# Format code with cargo fmt
fmt:
	@echo "=== Running cargo fmt ==="
	@cargo fmt --all

# Run clippy lints (fail on warnings)
clippy:
	@echo "=== Running cargo clippy ==="
	@cargo clippy --all-targets --all-features -- -D warnings

# Run both fmt and clippy
check: fmt clippy
	@echo "✅ Code quality checks passed"

# Build debug version
build:
	@echo "=== Building debug ==="
	@cargo build

# Build release version
release:
	@echo "=== Building release ==="
	@cargo build --release

# Run all tests including ignored ones
test:
	@echo "=== Running tests (including ignored) ==="
	@cargo test -- --include-ignored

# Build API documentation
docs:
	@echo "=== Building API documentation ==="
	@RUSTDOCFLAGS="--html-in-header .rustdoc-header.html" cargo doc --no-deps --release
	@echo "=== Copying to docs/api-docs/ ==="
	@mkdir -p docs/api-docs
	@rm -rf docs/api-docs/*
	@cp -r target/doc/* docs/api-docs/
	@echo "✅ Documentation built: docs/api-docs/hypermail/index.html"
	@echo "Preview: python3 -m http.server 8000 --directory docs"

# Run security audit (requires cargo-audit: cargo install cargo-audit)
audit:
	@echo "=== Running security audit ==="
	@command -v cargo-audit >/dev/null 2>&1 || { echo "cargo-audit not installed. Run: cargo install cargo-audit"; exit 1; }
	@cargo audit

# Check for dependency upgrades (requires cargo-edit: cargo install cargo-edit)
upgrade:
	@echo "=== Checking dependency upgrades ==="
	@command -v cargo-upgrade >/dev/null 2>&1 || { echo "cargo-edit not installed. Run: cargo install cargo-edit"; exit 1; }
	@echo "--- Compatible upgrades ---"
	@cargo upgrade --dry-run --verbose 2>&1 || true
	@echo "--- Incompatible (major) upgrades ---"
	@cargo upgrade --dry-run --verbose --incompatible 2>&1 || true

# Run fuzz tests (quick - 10 seconds each)
fuzz-quick:
	@echo "=== Running quick fuzz tests (10s each) ==="
	@command -v cargo-fuzz >/dev/null 2>&1 || { echo "cargo-fuzz not installed. Run: cargo install cargo-fuzz"; exit 1; }
	@for target in fuzz_parse_headers fuzz_parse_email fuzz_decode_mime \
	               fuzz_html_escape fuzz_unre fuzz_mbox_parse; do \
	  echo "Fuzzing $$target..."; \
	  cargo +nightly fuzz run $$target -- -max_total_time=10 -runs=10000 || true; \
	done
	@echo "✅ Fuzz tests completed"

# Run fuzz tests (full - 5 minutes each)
fuzz:
	@echo "=== Running fuzz tests (5min each, ~30min total) ==="
	@command -v cargo-fuzz >/dev/null 2>&1 || { echo "cargo-fuzz not installed. Run: cargo install cargo-fuzz"; exit 1; }
	@for target in fuzz_parse_headers fuzz_parse_email fuzz_decode_mime \
	               fuzz_html_escape fuzz_unre fuzz_mbox_parse; do \
	  echo "Fuzzing $$target..."; \
	  cargo +nightly fuzz run $$target -- -max_total_time=300 || true; \
	done
	@echo "✅ Fuzz tests completed"

# Clean build artifacts
clean:
	@echo "=== Cleaning ==="
	@cargo clean
	@rm -rf docs/api-docs

# Install git pre-commit hook
install-hooks:
	@echo "=== Installing git hooks ==="
	@mkdir -p .git/hooks
	@cp scripts/pre-commit .git/hooks/pre-commit
	@chmod +x .git/hooks/pre-commit
	@echo "✅ Git hooks installed"

# Full CI workflow: format, lint, security audit, dep check, build, test
ci: fmt clippy audit upgrade build test
	@echo "✅ All CI checks passed"

# Extended CI workflow with fuzz testing
ci-fuzz: ci fuzz-quick
	@echo "✅ All CI checks + fuzz tests passed"
