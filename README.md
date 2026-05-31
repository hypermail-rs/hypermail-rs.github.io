# hypermail-rs

A complete Rust rewrite of [Hypermail](https://www.hypermail-project.org/) — converts UNIX mbox files to cross-referenced HTML archives.

[![License: GPL-2.0-or-later](https://img.shields.io/badge/License-GPL--2.0--or--later-blue.svg)](https://www.gnu.org/licenses/gpl-2.0)

## Maintainer

hypermail-rs (Rust reimplementation) is maintained by:

- **Akis Karnouskos** — author of the Rust rewrite (2026)

## Attribution

hypermail-rs is a complete rewrite of the original Hypermail in Rust. This project would not exist without the pioneering work of the original Hypermail team.

### Original Hypermail Authors and Maintainers

**Creator (1994):**
- **Tom Gruber** — original Lisp prototype at Enterprise Integration Technologies (EIT)

**C Implementation:**
- **Kevin Hughes** (kevinh@kevcom.com) — C rewrite and initial maintainer at EIT

**Long-term Maintainer:**
- **Kent Landfield** (kent@landfield.com) — maintainer through versions 2.0–2.4

**Major Contributors:**
- **Daniel Stenberg** — MIME multipart handling, RFC 2047 decoding, trio library integration
- **Peter McCluskey** — lead developer; implemented `linkquotes`, `folder_by_date`, threading improvements
- **Jose Kahan** — early MIME support and internationalization
- **Stian Soiland** — thread index enhancements
- **Jim Meyering** — code quality and portability improvements
- **and other contributors** — see the original Hypermail Changelog for the full list

The original Hypermail project page is <https://www.hypermail-project.org/>.

hypermail-rs maintains the same GPL-2.0-or-later license and aims to be a drop-in replacement with expanded functionality. However, this is a complete rewrite in Rust — no original C code is included. New features have been added and backwards compatibility is not extensively tested or guaranteed.

For the complete list of contributors to the original Hypermail project, see the original Hypermail Changelog distributed with the upstream sources.

## Overview

hypermail-rs reads one or more mailbox files (or stdin) and produces a set of interlinked HTML pages: per-message pages with navigation, plus date/subject/author/thread index pages. It is a drop-in replacement for the original C hypermail, rewritten in safe Rust with expanded functionality.

## Features

- Full mbox-to-HTML conversion with incremental update support
- 189 languages (ISO 639-1/2 + IETF BCP 47 `x-*` private-use subtags)
- RFC 2231 attachment filename decoding (charset + continuation)
- RFC 3676 `format=flowed` rendering
- MIME multipart handling (`text/plain` preferred over HTML in alternatives)
- Thread building via `In-Reply-To`, `References`, and subject matching
- Multiple index types: date, subject, author, thread
- Security hardened: XSS prevention, CSP headers, path traversal protection
- Configurable via `.hmrc` files (compatible with original hypermail config format)
- Header cache (`.hm2index`) for fast incremental updates (activated with `-g`)
- Quote attribution linking and collapsible nested quotes
- Attachment extraction with inline image display
- Full-text search index (`search_index.txt`) generated alongside archives
- Platform: Linux, macOS (Windows partial — no file locking)

## Installation

```sh
cargo build --release
```

The binary is at `target/release/hypermail`.

## Documentation

- **Website**: https://hypermail-rs.github.io
- **API Documentation**: https://hypermail-rs.github.io/api-docs/hypermail/

## Usage

```sh
# Convert a mailbox file to HTML in output/
hypermail -m mailbox.mbox -d output/ -l "My Archive"

# Use a configuration file
hypermail -c archive.hmrc

# Read from stdin (pipe from fetchmail, procmail, etc.)
cat mailbox.mbox | hypermail -i -d output/ -l "Archive"

# Incremental update (append new messages only)
hypermail -u -m new-messages.mbox -d output/

# Overwrite existing archive
hypermail -x -m mailbox.mbox -d output/ -l "Fresh Archive"
```

### Key CLI options

| Flag | Long | Description |
|------|------|-------------|
| `-a` | `--archives` | URL for 'Other mail archives' link |
| `-A` | `--append` | Append mbox output to a parallel mailbox file |
| `-b` | `--about` | URL for 'About this archive' link |
| `-c` | `--config` | Configuration file (.hmrc) |
| `-d` | `--dir` | Output directory |
| `-g` | `--gdbm` | Use `.hm2index` header cache |
| `-i` | `--stdin` | Read from standard input |
| `-l` | `--label` | Archive name/title |
| `-L` | `--language` | Language code (e.g. `en`, `de`, `ja`) |
| `-m` | `--mbox` | Mailbox file to read |
| `-M` | `--metadata` | Use metadata files for attachments |
| `-n` | `--hmail` | List submission address |
| `-N` | `--nonsequential` | Use hash-based filenames |
| `-o` | `--set` | Set config option (e.g. `-o showhtml=2`) |
| `-p` | `--progress` | Show progress output |
| `-s` | `--suffix` | HTML file suffix (default: html) |
| `-T` | `--indextables` | Use index tables |
| `-u` | `--update` | Incremental update |
| `-v` | `--verbose` | Show configuration variable values and exit |
| `-V` | `--version` | Print version and exit |
| `-x` | `--overwrite` | Overwrite existing files |
| `-X` | `--xml` | Write HAOF XML archive overview file |
| `-0` | N/A | Delete message numbers |
| `-1` | `--readone` | Only one message in input |
| N/A | `--no-generator` | Suppress 'Generated by hypermail-rs' footer |
| N/A | `--warnings` | Show configuration warnings |

## Configuration

hypermail-rs reads `.hmrc` configuration files in the same format as the original hypermail. Example:

```ini
# archive.hmrc
mbox = /var/mail/list.mbox
dir = /var/www/archive/
label = Development Mailing List
language = en
defaultindex = thread
overwrite = 1

# Security
spamprotect = 1
antispam_at = _at_

# Appearance
showhtml = 2
indextable = 1
showreplies = 1
linkquotes = 1

# Attachments
attachmentsindex = 1
inline_types = image/gif image/jpeg image/png

# Folders
folder_by_date = %Y-%m
monthly_index = 1
```

Configuration can also be set via CLI with `-o key=value`. CLI options override file settings.

### Important config options

| Option | Type | Description |
|--------|------|-------------|
| `mbox` | string | Path to input mailbox |
| `dir` | string | Output directory |
| `label` | string | Archive title |
| `language` | string | UI language code |
| `defaultindex` | string | Default index page (`date`, `subject`, `author`, `thread`) |
| `overwrite` | bool | Overwrite existing message files |
| `spamprotect` | bool | Obfuscate email addresses |
| `showhtml` | int | HTML rendering (0=strip, 1=proportional, 2=full conversion) |
| `folder_by_date` | string | Split into date-based folders (strftime format) |
| `msgsperfolder` | int | Maximum messages per subdirectory folder |
| `attachmentsindex` | bool | Generate an attachment index page listing messages with MIME attachments |
| `uselock` | bool | Use file locking (Unix only) |
| `nonsequential` | bool | Hash-based filenames instead of sequential numbers |
| `usegdbm` | bool | Use `.hm2index` header cache for fast incremental updates |
| `antispamdomain` | string | Replace email domain in output (e.g., `user@example.com` → `user@privacy.invalid`) |
| `monthly_index` | bool | Generate per-month summary index pages |
| `yearly_index` | bool | Generate per-year summary index pages |
| `linkquotes` | bool | Link quoted text back to original messages |

## Template variables

When using custom HTML templates (`mhtmlheader`, `mhtmlfooter`, `ihtmlheader`, `ihtmlfooter`), the following `%x` escape sequences are available for printfile-style substitution:

| Code | Description |
|------|-------------|
| `%%` | Literal `%` character |
| `%a` | Other-archives URL (`archives` config option) |
| `%A` | Author `<meta name="Author">` tag (message pages only) |
| `%b` | About-archive URL (`about` config option) |
| `%c` | Charset `<meta>` tag |
| `%d` | Human-readable date string (plain text) |
| `%D` | Date `<meta name="Date">` tag (message pages only) |
| `%e` | Author email address (plain text) |
| `%f` | Current page filename |
| `%g` | Current date/time string |
| `%G` | Two-letter language code |
| `%h` | Hypermail homepage URL |
| `%i` | Message-ID |
| `%j` | Localized "Subject:" label (e.g., "Betreff:" in German, "Θέμα:" in Greek) |
| `%k` | Localized "Date:" label (e.g., "Datum:" in German, "Ημερομηνία:" in Greek) |
| `%l` | Archive label (`label` config option) |
| `%m` | Mailto address (`mailto` config option) |
| `%n` | Author display name (plain text, no markup) |
| `%N` | Author name + email as linked HTML |
| `%p` | Program name (`hypermail-rs`) |
| `%s` | Subject (HTML-escaped, with `stripsubject` applied) |
| `%S` | Subject `<meta name="Subject">` tag (message pages only) |
| `%t` | Relative path back to top-level index |
| `%u` | Version link: `<a href="...">hypermail-rs VERSION</a>` |
| `%v` | Version string |
| `%w` | Localized "Generated by" text |
| `%y` | Localized "Author:" label (e.g., "Autor:" in German, "Συγγραφέας:" in Greek) |
| `\n` | Newline |
| `\t` | Tab |

The internal default templates use `%COOKIE_NAME%` substitution for the same values (e.g., `%TITLE%`, `%ARTICLE%`, `%STYLESHEET%`).

## Language support

hypermail-rs ships with 189 compiled-in locale files covering all ISO 639-1 codes plus 7 IETF BCP 47 `x-*` private-use languages (Klingon, Lojban, Na'vi, etc.).

Set the language with `language = CODE` in your `.hmrc` file or `-L CODE` on the command line.

See [LANGUAGES.md](LANGUAGES.md) for the full list.

## Security

hypermail-rs is designed for safely archiving untrusted email input:

- All user-controlled content is HTML-escaped before output
- MIME type allowlist prevents script injection via `data:` URIs
- SVG is excluded from inline images (can contain `<script>`)
- Template substitution uses single-pass to prevent second-order injection
- `Content-Security-Policy` meta tags in generated pages
- Path traversal protection on attachment filenames
- 0 known CVEs in dependency tree (`cargo audit` clean)

See [SECURITY.md](SECURITY.md) for the full security review.

## Changes from Hypermail 2.4

### New functionality

- **189 languages** — all ISO 639-1/2 codes plus IETF BCP 47 private-use (`x-klingon`, `x-quenya`, etc.) compiled in at build time; original had ~15
- **RFC 2231** — full multi-part MIME parameter decoding (charset + continuation) for attachment filenames
- **RFC 3676** — `format=flowed` text rendering with proper line joining
- **Dark/light mode** — automatic via `prefers-color-scheme` plus a manual toggle button with `localStorage` persistence
- **Accessibility** — skip-to-content link, ARIA landmarks, focus-visible outlines, `prefers-contrast:more`, `prefers-reduced-motion`, responsive mobile layout, print stylesheet
- **Progress bar** (`-p`) — animated progress with percentage, count, and elapsed time
- **Content Security Policy** — meta tag on all generated pages
- **HAOF XML** export (`-X`) — machine-readable archive overview
- **Configurable max message size** — prevents memory exhaustion on malformed mbox files
- **Dynamic `lang` attribute** — HTML pages use the configured language code
- **Full-text search index** — `search_index.txt` generated for client-side search

### Security hardening (vs C hypermail)

- No buffer overflows (Rust memory safety)
- Systematic HTML escaping on all user content
- MIME type allowlist for inline images (SVG excluded)
- Single-pass template substitution (prevents second-order injection)
- Path traversal protection on attachment filenames
- `<script>` detection warning in custom templates

### Architecture improvements

| Aspect | Hypermail 2.4 (C) | hypermail-rs (Rust) |
|--------|-------------------|-------------------------|
| Memory safety | Manual malloc/free | Rust ownership model |
| Character encoding | ISO-8859-1 centric | Full UTF-8 throughout |
| MIME decoding | Custom charset tables | `encoding_rs` (WHATWG-compliant) |
| Build system | autotools + configure | `cargo build` (single command) |
| Dependencies | gdbm, pcre (optional C libs) | Pure Rust (no C dependencies) |
| Concurrency safety | Global variables | No mutable statics |
| Config parsing | Custom parser | Compatible format, stricter validation |
| Testing | None | Automated test suite |

### Removed/deprecated

- `-t` (tables) — accepted for compatibility but has no effect
- `showhr` — deprecated in the original C version; accepted in config but has no effect and emits a deprecation warning. Remove from your config file.
- `usetable` — deprecated in the original C version; accepted in config but has no effect and emits a deprecation warning. Remove from your config file.
- `body` — deprecated in the original C version; accepted in config but has no effect and emits a deprecation warning. Remove from your config file.

### Config key aliases

- `htmlheaderfile` — shorthand that sets both `ihtmlheaderfile` (index pages) and `mhtmlheaderfile` (message pages) to the same file. Equivalent to setting both individually.
- `htmlfooterfile` — shorthand that sets both `ihtmlfooterfile` and `mhtmlfooterfile` to the same file.

### Known gaps vs. original Hypermail 2.4

| Feature | Status |
|---------|--------|
| `mbox_shortened` | Constraints enforced (requires `usegdbm=1`, `increment=0`); actual skip-initial-messages logic is not implemented — the option is accepted and validated but the mbox is always read from the beginning |
| GDBM cache format | The `.hm2index` cache is a custom binary format; it is **not** binary-compatible with C hypermail's GDBM files — migrating requires a full archive rebuild |

#### GDBM cache

hypermail-rs implements its own binary header cache format (`.hm2index`) using the same `-g`/`usegdbm` flag. This is **not** binary-compatible with the original C hypermail's GDBM files. If you have an existing GDBM cache from C hypermail you must rebuild the archive from scratch.

## Building from source

### Requirements

- Rust 1.91+ (uses `floor_char_boundary()` stabilized in 1.91)
- No external C libraries required

### Build

```sh
git clone https://github.com/hypermail-rs/hypermail-rs.github.io.git
cd hypermail-rs.github.io
cargo build --release
```

### Test

```sh
cargo test
```

### Install

```sh
cargo install --path .
```

## License

GPL-2.0-or-later. See [LICENSE](LICENSE) for details.
