//! Fuzz target for the GeoIP database JSON parser.
//!
//! Ensures that `replace_geoip_db_from_json` never panics on arbitrary input,
//! including invalid CIDR notation, oversized prefixes, and malformed IPv4 addresses.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        // Must never panic — only return Ok/Err.
        let _ = core::rules_engine::replace_geoip_db_from_json(input);
    }
});
