# Contributing to Hypermail-rs

Thank you for contributing to hypermail-rs!

## Acknowledgment of Original Work

hypermail-rs is a complete Rust rewrite of the original Hypermail project, which was created by Tom Gruber at Enterprise Integration Technologies in 1994, rewritten in C by Kevin Hughes, and maintained by Kent Landfield and other contributors. This project builds on their pioneering work in email archiving and web-based collaboration tools.

We honor the legacy of the original Hypermail team and all contributors who shaped the project. While hypermail-rs is a complete rewrite with no original C code, it preserves the core concepts, configuration format, and GPL licensing of the original.

## Development Setup

### Prerequisites
- Rust 1.91+ (install from https://rustup.rs/)
- Git

### Clone and Build
```bash
git clone https://github.com/hypermail-rs/hypermail-rs.github.io.git
cd hypermail-rs.github.io
make build
```

## Code Quality

We use automated tools to maintain code quality:

### Formatting
All code must be formatted with `rustfmt`:
```bash
make fmt
# or
cargo fmt --all
```

### Linting
All code must pass `clippy` without warnings:
```bash
make clippy
# or
cargo clippy --all-targets --all-features -- -D warnings
```

### Combined Check
Run both formatting and linting:
```bash
make check
```

### Testing
Run the full test suite:
```bash
make test
# or
cargo test
```

## Git Workflow

### Install Pre-commit Hooks
We provide a pre-commit hook that automatically runs formatting and linting checks:
```bash
make install-hooks
```

This will:
- Run `cargo fmt` and auto-format your code
- Run `cargo clippy` and fail if there are warnings
- Prevent commits with formatting/linting issues

### Commit Guidelines
1. Write clear, descriptive commit messages
2. Keep commits focused on a single logical change
3. Run `make check` before committing
4. Ensure all tests pass with `make test`

## Continuous Integration

Our CI pipeline (GitHub Actions) runs on every push and pull request:
- ✅ Formatting check (`cargo fmt --check`)
- ✅ Clippy lints (`cargo clippy`)
- ✅ Tests on Ubuntu, macOS, and Windows
- ✅ Release build verification
- ✅ Code coverage analysis

All checks must pass before a PR can be merged.

## Common Tasks

### Full CI workflow locally
```bash
make ci
```

This runs:
1. `cargo fmt` - Format code
2. `cargo clippy` - Lint code
3. `cargo build` - Build debug version
4. `cargo test` - Run all tests

### Build release version
```bash
make release
# or
cargo build --release
```

### Clean build artifacts
```bash
make clean
```

## Testing Guidelines

### Running Specific Tests
```bash
# Run tests matching a pattern
cargo test test_decode_body

# Run tests with output
cargo test -- --nocapture

# Run tests in a specific module
cargo test mime::tests::
```

### Writing Tests
- Add unit tests in the same file as the code (in a `#[cfg(test)] mod tests { ... }` block)
- Add integration tests in `tests/` directory
- Ensure tests are deterministic and don't depend on external state
- Use descriptive test names that explain what is being tested

### Test Coverage
- Aim for high test coverage, especially for charset detection and MIME parsing
- Add tests for edge cases and error conditions
- Document expected behavior in test assertions

## Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run `make ci` to verify all checks pass
5. Commit your changes (`git commit -m 'Add amazing feature'`)
6. Push to your fork (`git push origin feature/amazing-feature`)
7. Open a Pull Request

### PR Requirements
- All CI checks must pass (formatting, clippy, tests)
- Add tests for new functionality
- Update documentation if needed
- Keep PRs focused and atomic

## Project Structure

```
hypermail-rs/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library exports
│   ├── config.rs        # Configuration
│   ├── headers.rs       # Email header parsing
│   ├── mime.rs          # MIME body parsing
│   ├── html.rs          # HTML generation
│   ├── templates.rs     # HTML templates
│   ├── txt2html.rs      # Text to HTML conversion
│   └── date.rs          # Date parsing
├── tests/               # Integration tests
├── scripts/             # Development scripts
│   └── pre-commit       # Git pre-commit hook
├── .github/
│   └── workflows/       # CI/CD workflows
│       └── ci.yml       # GitHub Actions CI
├── Makefile             # Build automation
└── Cargo.toml           # Rust package config
```

## License

hypermail-rs is licensed under the **GNU General Public License v2.0 or later**
(GPL-2.0-or-later). This is required because hypermail-rs is a derivative work of
the original C Hypermail, which carries the same license.

**By submitting a contribution (pull request, patch, or any other change) to this
repository, you agree that your contribution is licensed under GPL-2.0-or-later,
consistent with the project license.** You confirm that you have the right to make
that grant — i.e. the code is your own original work, or you have obtained the
necessary permissions from the original author(s).

If your contribution includes code from a third-party source, please note that
clearly in the pull request so it can be reviewed for license compatibility before
merging.

## Questions?

If you have questions or need help, please:
- Open an issue on GitHub
- Check existing issues and pull requests
- Read the README.md for project overview

Thank you for contributing!
