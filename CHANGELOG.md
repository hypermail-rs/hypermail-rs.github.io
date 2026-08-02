# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- `delete_older`/`delete_newer`: empty config values are now treated as unset instead of emitting a spurious "Could not parse date" warning on every run.
- `-t`/`--tables` now correctly maps to `indextable` (same as `-T`) instead of being a no-op.
- Absolute-path guard for `folder_by_date`/`latest_folder` now correctly rejects Unix-style absolute paths on Windows.

### Added

- `require_msgids` and `discard_dup_msgids` config options are now honored during mbox processing.
- `from_date`/`from_date_str` populated from the Date header for nonsequential filename hashing.
- Multipart MIME nesting depth limit to prevent stack overflow on crafted messages.
- Iterative BST insert/traversal/drop in the email store to avoid stack overflow on large archives.

### Security

- CSP `script-src` now pins the inline theme/a11y script via a `sha256` hash instead of `unsafe-inline`.
- Archive title is HTML-escaped before being substituted into page `<title>`/headings.
- Inline image data URIs validate the base64 payload alphabet to prevent HTML attribute breakout.

## [1.0.0] - 2026-06-16

### Project Rename and Versioning

**IMPORTANT**: This release marks the formal recognition that hypermail-rs is a **new Rust implementation** of the original Hypermail (C version), not a direct continuation. Therefore:

- Package name: `hypermail-rs` (distinguishes from original)
- Executable name: `hypermail` (drop-in replacement compatibility)
- Version: `1.0` (new project lifecycle)
- Configuration: Maintains compatibility with Hypermail 2.4.0 configs

### Added

- **Subject-based threading**: Automatically threads messages with Re:/Fwd:/AW:/SV: prefixes even without In-Reply-To headers
- Smart charset detection for mislabeled ISO-8859-1 → ISO-8859-7 (Greek) content
- Comprehensive test suite including subject threading tests
- GitHub Actions CI/CD pipeline with formatting, clippy, and coverage
- Pre-commit hooks for formatting and linting
- Makefile for common development tasks (fmt, clippy, test, ci)
- HTML5 semantic thread structure with nested `<ul>/<li>` instead of flat divs
- Dark mode support with `prefers-color-scheme` media queries
- High contrast mode support with `prefers-contrast`
- Reduced motion support for accessibility
- Skip-to-content link for screen readers
- Content Security Policy (CSP) headers
- Noto Sans font via Google Fonts CDN with system font fallbacks
- Documentation: README.md (with attribution to original Hypermail), CONTRIBUTING.md, CHANGELOG.md, LANGUAGES.md, SECURITY.md, LICENSE (GPL-2.0+) and NOTICE (attribution)

### Changed

- **Version numbering**: Reset to 1.0 to reflect new implementation
- **Threading depth**: Default `thrdlevels` increased from 4 to 50 for deeper thread display
- **Thread structure**: Changed from flat divs to semantic nested `<ul>/<li>` HTML
- **Charset detection**: Uses 30% Greek Unicode threshold to prevent false positives on Latin-1 text
- **Font stack**: Noto Sans via Google Fonts with system font fallbacks
- **MIME parsing**: Improved multipart handling with better image embedding
- **Code quality**: All clippy warnings fixed (strict mode with `-D warnings`)

### Fixed

- Mislabeled ISO-8859-1 messages with Greek content now display correctly
- Missing fields in Config::default() implementation (filter_out, filter_require, prefered_types, ignore_types)
- X-No-Archive behavior now matches original hypermail (delete_level=3 shows content)
- Inline images now properly embedded as data URIs
- RFC 2047 header decoding for Greek and other non-ASCII text
- Thread display now shows full depth (not limited to 4 levels)


---

## Historical Note: Relationship to Original Hypermail

This project is inspired by and maintains configuration compatibility with the original **Hypermail** email archiver:

- **Original Hypermail**: C implementation
- **Authors**: Tom Gruber, Kevin Hughes, Kent Landfield, Peter McCluskey, Daniel Stenberg, Jose Kahan, and other contributors
- **Project page**: https://www.hypermail-project.org/
- **License**: GPL-2.0+

**Hypermail-rs** is a Rust reimplementation that adds modern features while honoring the original's design principles. We start versioning at 1.0 to reflect this is a new codebase with its own lifecycle, while maintaining compatibility where appropriate.

For users migrating from Hypermail 2.4.0:
- Configuration files are compatible
- Command-line options are compatible
- HTML output format is enhanced but similar
- New features can be disabled for strict compatibility

---

[1.0.0]: https://github.com/hypermail-rs/hypermail-rs.github.io/releases/tag/v1.0.0
