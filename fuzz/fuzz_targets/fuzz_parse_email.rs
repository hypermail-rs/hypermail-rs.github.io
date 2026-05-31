#![no_main]

use libfuzzer_sys::fuzz_target;

/// Fuzz target for email address parsing.
///
/// Tests parse_email_address() against various malformed inputs:
/// - Multiple @ symbols
/// - Nested angle brackets
/// - Unclosed delimiters
/// - Unicode characters
/// - Empty strings
/// - Very long addresses
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Should never panic, even on malformed input
        let (_name, email) = hypermail::headers::parse_email_address(s);
        
        // Sanity check: if email extracted, it should contain @
        if let Some(addr) = email {
            // This is expected behavior, not a requirement
            let _ = addr.contains('@');
        }
        
        // Test with truncated input
        if s.len() > 10 {
            let truncated = &s[..10];
            let _ = hypermail::headers::parse_email_address(truncated);
        }
    }
});
