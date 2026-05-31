# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.0.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security issue in hypermail-rs, please report it responsibly.

### How to Report

**Please DO NOT open a public GitHub issue for security vulnerabilities.**

Instead, please email security details to: **hypermail-rs-security@users.noreply.github.com**

Include in your report:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### What to Expect

- **Acknowledgment**: We will acknowledge receipt of your report within 48 hours
- **Updates**: We will provide regular updates on our progress
- **Timeline**: We aim to release a fix within 30 days for critical vulnerabilities
- **Credit**: With your permission, we will credit you in the security advisory

## Security Features

hypermail-rs implements several security measures:

### Input Validation
- **Message Size Limits**: Maximum 100MB per message, 10MB per line
- **URL Length Limits**: Maximum 4096 characters
- **Subject Length Limits**: Maximum 2048 characters for threading
- **Path Validation**: Rejects paths containing `..` to prevent traversal attacks

### HTML Output Security
- **XSS Prevention**: All user-controlled content is HTML-escaped
- **MIME Type Allowlist**: Only safe content types allowed for inline display
- **Content Security Policy**: Generated pages include CSP headers
- **Attribute Escaping**: All HTML attributes properly escaped

### Memory Safety
- **Safe Rust**: Written in 100% safe Rust (except for one documented `unsafe` block with safety comments)
- **No Buffer Overflows**: Rust's memory safety guarantees eliminate buffer overflow vulnerabilities
- **Dependency Auditing**: Regular `cargo audit` checks for vulnerable dependencies

### Regular Expression Safety
- **ReDoS Protection**: Input length limits prevent regex denial-of-service
- **Static Compilation**: Regex patterns compiled once at startup using `LazyLock`

## Security Auditing

We maintain security through:
- Regular dependency updates via `cargo update`
- Continuous monitoring with `cargo audit`
- Fuzzing tests for critical parsers (mbox, MIME, headers)
- Static analysis with `cargo clippy`

## Dependency Policy

We track and update dependencies regularly. Run these commands to check:

```bash
cargo audit              # Check for known vulnerabilities
cargo outdated           # Check for outdated dependencies
```

Current status: **0 known vulnerabilities** (verify with `cargo audit`)

## License

hypermail-rs is licensed under GPL-2.0-or-later. Security fixes are provided in accordance with this license.
