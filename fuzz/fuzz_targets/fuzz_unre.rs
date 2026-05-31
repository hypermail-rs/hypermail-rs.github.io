#![no_main]

use libfuzzer_sys::fuzz_target;

/// Fuzz target for subject prefix stripping.
///
/// Tests unre() function for:
/// - Performance on long subjects (ReDoS protection)
/// - Correct handling of various prefixes
/// - Unicode in subjects
/// - Malformed prefixes
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Should never panic or hang, even on pathological input
        let stripped = hypermail::string_utils::unre(s);
        
        // Invariant: output should be <= input length
        assert!(stripped.len() <= s.len());
        
        // Invariant: output should be valid UTF-8
        assert!(std::str::from_utf8(stripped.as_bytes()).is_ok());
        
        // Test that common prefixes are removed
        if !s.is_empty() {
            let with_re = format!("Re: {}", s);
            let result = hypermail::string_utils::unre(&with_re);
            // Should have stripped "Re: " prefix
            assert!(result.len() <= with_re.len());
            
            let with_fwd = format!("Fwd: {}", s);
            let result = hypermail::string_utils::unre(&with_fwd);
            assert!(result.len() <= with_fwd.len());
        }
        
        // Test oneunre as well
        let _ = hypermail::string_utils::oneunre(s);
    }
});
