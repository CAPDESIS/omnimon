//! Fuzz target for the AI rules JSON parser.
//!
//! Ensures that `upsert_rules_from_ai_json` never panics on arbitrary input,
//! including malformed JSON, missing fields, and unexpected types.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        // Must never panic — only return Ok/Err.
        let _ = core::rules_engine::upsert_rules_from_ai_json(input);
    }
});
