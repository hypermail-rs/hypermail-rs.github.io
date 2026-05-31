#![no_main]

use libfuzzer_sys::fuzz_target;

/// Fuzz target for mbox message parsing.
///
/// Tests the complete mbox parsing pipeline:
/// - Message boundary detection
/// - Header/body separation
/// - Size limit enforcement
/// - Malformed mbox handling
fuzz_target!(|data: &[u8]| {
    // Create mbox reader from fuzz input
    let cursor = std::io::Cursor::new(data);
    let mut reader = hypermail::mbox::MboxReader::new(
        cursor,
        hypermail::mbox::MboxFormat::MboxO
    );
    
    // Try to parse up to 10 messages
    // (limit iterations to prevent timeout on valid huge inputs)
    for _ in 0..10 {
        match reader.next() {
            None => break,  // End of mbox
            Some(Ok(_msg)) => {
                // Successfully parsed a message
                // In real code, would process here
            },
            Some(Err(_e)) => {
                // Parse error - this is acceptable for malformed input
                // The important thing is it didn't panic
                break;
            }
        }
    }
});
