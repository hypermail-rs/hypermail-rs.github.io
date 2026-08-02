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
- **Path Validation**: Rejects paths containing `..` or absolute paths in `folder_by_date` and `latest_folder`, to prevent traversal/symlink-escape via config

### HTML Output Security
- **XSS Prevention**: User-controlled content (subjects, bodies, titles) is HTML-escaped before output
- **Inline image markers**: MIME type allowlist plus base64 alphabet validation (blocks attribute breakout)
- **MIME Type Allowlist**: Only safe content types allowed for inline display (SVG excluded)
- **Content Security Policy**: Generated pages include CSP headers; `script-src` is pinned to a sha256 hash of the fixed inline theme/accessibility script (no `unsafe-inline` for scripts), so injected `<script>` content anywhere else on the page is still blocked by the browser
- **Attribute Escaping**: HTML attributes properly escaped

### Memory Safety
- **Safe Rust**: Written in 100% safe Rust (except for one documented `unsafe` block with safety comments)
- **No Buffer Overflows**: Rust's memory safety guarantees eliminate buffer overflow vulnerabilities
- **Dependency Auditing**: Regular `cargo audit` checks for vulnerable dependencies
- **Stack-safe tree operations**: The subject/author/date index is a simple BST that degenerates to a linked list on near-sorted (the common case) input; insert, traversal, and drop are all iterative (explicit stack / work list) rather than recursive, so large archives cannot trigger a stack-overflow abort

### Regular Expression Safety
- **ReDoS Protection**: Input and pattern length limits on filter regexes; URL/subject length caps
- **Static Compilation**: Built-in regex patterns compiled once at startup using `std::sync::LazyLock`
- **Multipart depth limit**: Nested multiparts capped to prevent stack exhaustion

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
