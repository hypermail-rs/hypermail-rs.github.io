#![no_main]

use libfuzzer_sys::fuzz_target;

/// Fuzz target for email header parsing.
///
/// This tests the robustness of parse_headers() against malformed,
/// malicious, or unexpected input. The parser should never panic
/// or crash regardless of input.
///
/// Test vectors include:
/// - Malformed headers (missing colons, invalid encoding)
/// - Extremely long header names/values
/// - Invalid UTF-8 sequences
/// - Control characters and null bytes
/// - Deeply nested MIME encoding
/// - Headers with no newlines
fuzz_target!(|data: &[u8]| {
    // Test 1: Parse as-is (may be invalid UTF-8)
    let _ = hypermail::headers::parse_headers(data);
    
    // Test 2: Parse valid UTF-8 subset
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = hypermail::headers::parse_headers(s.as_bytes());
        
        // Test 3: Parse individual header lines
        for line in s.lines() {
            let _ = hypermail::headers::parse_headers(line.as_bytes());
        }
    }
});
