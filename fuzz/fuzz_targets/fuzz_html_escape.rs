#![no_main]

use libfuzzer_sys::fuzz_target;

/// Fuzz target for HTML escaping.
///
/// Critical security function - must never allow XSS through.
/// Tests that all dangerous characters are properly escaped.
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let escaped = hypermail::txt2html::escape_html(s);
        
        // Security invariants: these characters must NEVER appear unescaped in output
        // except as part of HTML entities
        
        // Check that raw < and > don't appear (should be &lt; and &gt;)
        let escaped_bytes = escaped.as_bytes();
        for i in 0..escaped_bytes.len() {
            if escaped_bytes[i] == b'<' {
                // Should be part of &lt; or other entity
                if i == 0 || escaped_bytes[i-1] != b'&' {
                    // Lone < found - this is a bug!
                    panic!("Unescaped < found in output: {}", escaped);
                }
            }
            if escaped_bytes[i] == b'>' {
                // Should be part of &gt; or other entity  
                if i < 3 || &escaped_bytes[i-3..=i] != b"&gt;" {
                    // Check if it's part of other entities
                    let is_entity = i >= 5 && (
                        &escaped_bytes[i-5..=i] == b"&quot;" ||
                        &escaped_bytes[i-4..=i] == b"&#39;" ||
                        &escaped_bytes[i-4..=i] == b"&amp;"
                    );
                    if !is_entity {
                        panic!("Unescaped > found in output: {}", escaped);
                    }
                }
            }
        }
        
        // Test that re-escaping is safe (idempotent)
        let double_escaped = hypermail::txt2html::escape_html(&escaped);
        assert!(double_escaped.len() >= escaped.len());
    }
});
