# Fuzz Testing for Hypermail-rs

This directory contains fuzz testing targets for hypermail-rs using `cargo-fuzz` and `libFuzzer`.

## Overview

Fuzz testing automatically generates random inputs to discover bugs, panics, and security vulnerabilities that may not be caught by regular unit tests. This is especially important for parsing code that handles untrusted email data.

## Fuzz Targets

### 1. `fuzz_parse_headers` - Email Header Parsing
Tests `hypermail::headers::parse_headers()` against:
- Malformed headers (missing colons, invalid encoding)
- Extremely long header names/values
- Invalid UTF-8 sequences
- Control characters and null bytes
- Deeply nested MIME encoding

**Why it matters**: Headers are the first attack surface when processing emails.

### 2. `fuzz_parse_email` - Email Address Extraction
Tests `hypermail::headers::parse_email_address()` against:
- Multiple @ symbols
- Nested angle brackets
- Unclosed delimiters
- Unicode characters
- Very long addresses

**Why it matters**: Email address parsing is used throughout the codebase.

### 3. `fuzz_decode_mime` - RFC 2047 MIME Decoding
Tests `hypermail::headers::decode_mime_words()` against:
- Invalid base64 encoding
- Invalid quoted-printable encoding
- Unknown charsets
- Malformed =?...?= syntax
- Nested encoding

**Why it matters**: MIME decoding handles untrusted encoded content.

### 4. `fuzz_html_escape` - HTML Escaping (SECURITY CRITICAL)
Tests `hypermail::txt2html::escape_html()` against:
- XSS attack vectors
- Unicode tricks
- Malformed entities
- Double-escaping edge cases

**Why it matters**: This is the primary XSS defense. Any bug here is a critical security vulnerability.

### 5. `fuzz_unre` - Subject Prefix Stripping
Tests `hypermail::string_utils::unre()` against:
- Very long subjects (ReDoS protection)
- Various reply prefixes
- Unicode in subjects
- Malformed prefixes

**Why it matters**: Used in O(n²) threading loop, must not hang on pathological input.

### 6. `fuzz_mbox_parse` - Mbox Message Parsing
Tests `hypermail::mbox::MboxReader` against:
- Malformed mbox boundaries
- Missing headers/bodies
- Size limit enforcement
- Invalid message structure

**Why it matters**: The entry point for all email processing.

## Running Fuzz Tests

### Prerequisites

```bash
# Install cargo-fuzz (one-time setup)
cargo install cargo-fuzz

# Requires nightly Rust
rustup install nightly
```

### Run a Specific Fuzz Target

```bash
# Run for 60 seconds
cargo +nightly fuzz run fuzz_html_escape -- -max_total_time=60

# Run until crash found or Ctrl+C
cargo +nightly fuzz run fuzz_parse_headers

# Run with more memory (for mbox parsing)
cargo +nightly fuzz run fuzz_mbox_parse -- -rss_limit_mb=2048
```

### Run All Targets (Recommended)

```bash
# Run each target for 5 minutes
for target in fuzz_parse_headers fuzz_parse_email fuzz_decode_mime \
              fuzz_html_escape fuzz_unre fuzz_mbox_parse; do
  echo "=== Fuzzing $target ==="
  cargo +nightly fuzz run $target -- -max_total_time=300 || true
done
```

### Continuous Fuzzing (CI/CD)

```bash
# Short run for CI (10 seconds each)
for target in fuzz_*; do
  cargo +nightly fuzz run $target -- -max_total_time=10 -runs=10000
done
```

## Analyzing Crashes

If a fuzz target finds a crash:

```bash
# Crash artifacts are saved to fuzz/artifacts/<target>/
ls fuzz/artifacts/fuzz_html_escape/

# Reproduce the crash
cargo +nightly fuzz run fuzz_html_escape fuzz/artifacts/fuzz_html_escape/crash-abc123

# Minimize the crash input
cargo +nightly fuzz cmin fuzz_html_escape
cargo +nightly fuzz tmin fuzz_html_escape fuzz/artifacts/fuzz_html_escape/crash-abc123
```

## Coverage Reports

```bash
# Generate coverage report
cargo +nightly fuzz coverage fuzz_html_escape

# View coverage (requires llvm-cov)
llvm-cov show target/*/release/fuzz_html_escape \
  --format=html \
  -instr-profile=fuzz/coverage/fuzz_html_escape/coverage.profdata \
  > coverage.html
```

## Integration with CI

Add to `.github/workflows/fuzz.yml`:

```yaml
name: Fuzz Testing

on:
  schedule:
    - cron: '0 2 * * *'  # Daily at 2 AM
  workflow_dispatch:

jobs:
  fuzz:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        target:
          - fuzz_parse_headers
          - fuzz_parse_email
          - fuzz_decode_mime
          - fuzz_html_escape
          - fuzz_unre
          - fuzz_mbox_parse
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust nightly
        run: rustup install nightly
      
      - name: Install cargo-fuzz
        run: cargo install cargo-fuzz
      
      - name: Run fuzz target
        run: |
          cargo +nightly fuzz run ${{ matrix.target }} -- \
            -max_total_time=300 \
            -timeout=10
      
      - name: Upload artifacts
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: fuzz-artifacts-${{ matrix.target }}
          path: fuzz/artifacts/${{ matrix.target }}/
```

## Expected Results

### Security Targets
- `fuzz_html_escape`: Should NEVER find XSS bypass (0 crashes expected)
- `fuzz_decode_mime`: Should handle invalid encoding gracefully

### Robustness Targets
- `fuzz_parse_headers`: May find edge cases in malformed headers
- `fuzz_parse_email`: May find edge cases in address parsing
- `fuzz_unre`: Should complete quickly even on pathological input
- `fuzz_mbox_parse`: Should enforce size limits correctly

## Performance Expectations

| Target | Speed | Memory | Notes |
|--------|-------|--------|-------|
| `fuzz_parse_headers` | Fast (10k+ exec/sec) | Low | Simple parsing |
| `fuzz_parse_email` | Very fast (50k+ exec/sec) | Low | Minimal allocation |
| `fuzz_decode_mime` | Medium (5k exec/sec) | Medium | Base64 decoding |
| `fuzz_html_escape` | Very fast (100k+ exec/sec) | Low | Character iteration |
| `fuzz_unre` | Fast (20k+ exec/sec) | Low | Regex with truncation |
| `fuzz_mbox_parse` | Slow (100-1k exec/sec) | High | Full parsing pipeline |

## Corpus Management

```bash
# Merge interesting inputs from multiple runs
cargo +nightly fuzz cmin fuzz_html_escape

# Add seed corpus (optional)
mkdir -p fuzz/corpus/fuzz_html_escape
echo "<script>alert(1)</script>" > fuzz/corpus/fuzz_html_escape/xss1
echo "'; DROP TABLE users--" > fuzz/corpus/fuzz_html_escape/sqli1
```

## Troubleshooting

### "error: no fuzz targets found"
Make sure you're in the project root, not the `fuzz/` directory.

### "sanitizer errors"
This is expected - fuzzing uses sanitizers to find bugs. Investigate the error.

### "out of memory"
Increase RSS limit: `-rss_limit_mb=4096`

### "too slow"
- Reduce complexity in the fuzz target
- Use smaller max input size: `-max_len=1024`
- Optimize the code being fuzzed

## Best Practices

1. **Run regularly**: Fuzz tests should run overnight or in CI
2. **Minimize inputs**: Use `fuzz tmin` to create minimal reproducers
3. **Add regression tests**: Convert crashes to unit tests
4. **Monitor coverage**: Aim for >80% code coverage from fuzzing
5. **Update corpus**: Keep successful inputs as seed corpus

## References

- [cargo-fuzz documentation](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- [libFuzzer documentation](https://llvm.org/docs/LibFuzzer.html)
- [Fuzzing best practices](https://github.com/google/fuzzing/blob/master/docs/good-fuzz-target.md)

## Maintenance

- Review fuzz targets quarterly
- Update after major code changes
- Add new targets for new parsing code
- Clean old artifacts periodically

---

**Note**: Fuzzing requires nightly Rust and is CPU-intensive. For continuous fuzzing, consider using [OSS-Fuzz](https://github.com/google/oss-fuzz) for cloud-based fuzzing.
