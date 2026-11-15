//! Explicit opt-in. Also tested with debug assertions disabled.
use std::ffi::OsStr;

pub fn enabled(value: Option<&OsStr>) -> bool {
    cfg!(debug_assertions) && value == Some(OsStr::new("1"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn demo_requires_exact_opt_in_and_debug_build() {
        assert!(!enabled(None));
        for value in ["", "0", "true", " 1"] {
            assert!(!enabled(Some(OsStr::new(value))));
        }
        assert_eq!(enabled(Some(OsStr::new("1"))), cfg!(debug_assertions));
    }
}
