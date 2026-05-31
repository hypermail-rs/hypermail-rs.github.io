#![no_main]

use libfuzzer_sys::fuzz_target;

/// Fuzz target for RFC 2047 MIME word decoding.
///
/// Tests decode_mime_words() against:
/// - Invalid base64 encoding
/// - Invalid quoted-printable encoding
/// - Unknown charsets
/// - Malformed =?...?= syntax
/// - Nested encoding
/// - Extremely long encoded words
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Should handle any string without panicking
        let decoded = hypermail::headers::decode_mime_words(s);
        
        // The decoded output should be valid UTF-8
        assert!(decoded.is_ascii() || std::str::from_utf8(decoded.as_bytes()).is_ok());
        
        // Test with common MIME encoding patterns
        if !s.is_empty() {
            let with_prefix = format!("=?UTF-8?B?{}?=", s);
            let _ = hypermail::headers::decode_mime_words(&with_prefix);
            
            let with_qp = format!("=?UTF-8?Q?{}?=", s);
            let _ = hypermail::headers::decode_mime_words(&with_qp);
        }
    }
});
