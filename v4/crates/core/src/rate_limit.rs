//! Token Bucket rate limiter for IPC command protection.
//!
//! Prevents internal DDoS from rapid frontend invocations by throttling
//! critical commands (kill_process, AI calls, browser control).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

/// Per-bucket configuration: how many tokens and the refill rate.
#[derive(Debug, Clone, Copy)]
pub struct BucketConfig {
    /// Maximum number of tokens the bucket can hold.
    pub capacity: u32,
    /// Tokens added per second (fractional via integer math).
    pub refill_per_sec: f64,
}

impl BucketConfig {
    pub const fn new(capacity: u32, refill_per_sec: f64) -> Self {
        Self {
            capacity,
            refill_per_sec,
        }
    }
}

/// Predefined rate limit profiles for different IPC command categories.
pub mod profiles {
    use super::BucketConfig;

    /// Destructive actions: kill_process, kill_processes.
    /// 5 kills/sec burst, refills at 2/sec.
    pub const KILL: BucketConfig = BucketConfig::new(5, 2.0);

    /// AI calls: analyze_processes, analyze_context, ai_chat, validate_api_key.
    /// 3 calls burst, refills at 0.5/sec (1 every 2 seconds).
    pub const AI: BucketConfig = BucketConfig::new(3, 0.5);

    /// Browser control: close_browser_tab, focus_browser_tab.
    /// 10 actions/sec burst, refills at 5/sec.
    pub const BROWSER: BucketConfig = BucketConfig::new(10, 5.0);

    /// Cloud/config operations: save_ai_config, save_cloud_key.
    /// 3 operations burst, refills at 1/sec.
    pub const CONFIG: BucketConfig = BucketConfig::new(3, 1.0);
}

#[derive(Debug)]
struct TokenBucket {
    tokens: f64,
    capacity: u32,
    refill_per_sec: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(config: &BucketConfig) -> Self {
        Self {
            tokens: config.capacity as f64,
            capacity: config.capacity,
            refill_per_sec: config.refill_per_sec,
            last_refill: Instant::now(),
        }
    }

    /// Try to consume one token. Returns `true` if allowed, `false` if rate-limited.
    fn try_acquire(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        if elapsed > 0.0 {
            self.tokens = (self.tokens + elapsed * self.refill_per_sec).min(self.capacity as f64);
            self.last_refill = now;
        }
    }
}

static RATE_LIMITERS: OnceLock<Mutex<HashMap<&'static str, TokenBucket>>> = OnceLock::new();

fn limiters() -> &'static Mutex<HashMap<&'static str, TokenBucket>> {
    RATE_LIMITERS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Check if a command is allowed under its rate limit.
///
/// Returns `Ok(())` if the command is allowed, or `Err(message)` if rate-limited.
/// The bucket is created on first use with the given config.
pub fn check_rate_limit(bucket_name: &'static str, config: &BucketConfig) -> Result<(), String> {
    let mut guard = limiters()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let bucket = guard
        .entry(bucket_name)
        .or_insert_with(|| TokenBucket::new(config));

    if bucket.try_acquire() {
        Ok(())
    } else {
        Err(format!(
            "Rate limited: too many '{}' requests. Please wait before retrying.",
            bucket_name
        ))
    }
}

/// Reset all rate limiters. Useful for testing.
#[cfg(test)]
pub fn reset_all() {
    if let Ok(mut guard) = limiters().lock() {
        guard.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_BUCKET: BucketConfig = BucketConfig::new(3, 100.0);

    #[test]
    fn allows_requests_within_capacity() {
        reset_all();
        assert!(check_rate_limit("test_cap", &TEST_BUCKET).is_ok());
        assert!(check_rate_limit("test_cap", &TEST_BUCKET).is_ok());
        assert!(check_rate_limit("test_cap", &TEST_BUCKET).is_ok());
    }

    #[test]
    fn rejects_after_capacity_exhausted() {
        reset_all();
        let config = BucketConfig::new(2, 0.0); // no refill
        assert!(check_rate_limit("test_exhaust", &config).is_ok());
        assert!(check_rate_limit("test_exhaust", &config).is_ok());
        let result = check_rate_limit("test_exhaust", &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Rate limited"));
    }

    #[test]
    fn separate_buckets_are_independent() {
        reset_all();
        let config = BucketConfig::new(1, 0.0);
        assert!(check_rate_limit("bucket_a", &config).is_ok());
        assert!(check_rate_limit("bucket_b", &config).is_ok());
        // bucket_a exhausted but bucket_b still works
        assert!(check_rate_limit("bucket_a", &config).is_err());
        // bucket_b also exhausted now
        assert!(check_rate_limit("bucket_b", &config).is_err());
    }

    #[test]
    fn refills_over_time() {
        reset_all();
        let config = BucketConfig::new(1, 1000.0); // fast refill for test
        assert!(check_rate_limit("test_refill", &config).is_ok());
        assert!(check_rate_limit("test_refill", &config).is_err());
        // Wait for refill
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(check_rate_limit("test_refill", &config).is_ok());
    }

    #[test]
    fn does_not_exceed_capacity_on_long_wait() {
        reset_all();
        let config = BucketConfig::new(2, 50.0);
        // Exhaust all tokens
        assert!(check_rate_limit("test_cap_max", &config).is_ok());
        assert!(check_rate_limit("test_cap_max", &config).is_ok());
        assert!(check_rate_limit("test_cap_max", &config).is_err());

        // Long wait — should refill to capacity, not beyond
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(check_rate_limit("test_cap_max", &config).is_ok());
        assert!(check_rate_limit("test_cap_max", &config).is_ok());
        // Should be at 0 now, even if we waited long enough for >2 refills
        assert!(check_rate_limit("test_cap_max", &config).is_err());
    }

    #[test]
    fn error_message_contains_bucket_name() {
        reset_all();
        let config = BucketConfig::new(0, 0.0);
        let err = check_rate_limit("my_command", &config).unwrap_err();
        assert!(err.contains("my_command"));
    }

    #[test]
    fn predefined_profiles_have_sane_values() {
        const {
            assert!(profiles::KILL.capacity > 0);
            assert!(profiles::KILL.refill_per_sec > 0.0);
            assert!(profiles::AI.capacity > 0);
            assert!(profiles::AI.refill_per_sec > 0.0);
            assert!(profiles::BROWSER.capacity > 0);
            assert!(profiles::BROWSER.refill_per_sec > 0.0);
            assert!(profiles::CONFIG.capacity > 0);
            assert!(profiles::CONFIG.refill_per_sec > 0.0);
        }
    }
}
