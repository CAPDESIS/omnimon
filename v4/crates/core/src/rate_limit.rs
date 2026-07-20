//! Token Bucket rate limiter for IPC command protection.
//!
//! Prevents internal DDoS from rapid frontend invocations by throttling
//! critical commands (kill_process, AI calls, browser control).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

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
    /// 10 kills burst, refills at 5/sec — generous enough for AI batch kills
    /// and user-initiated multi-kills without silently blocking.
    pub const KILL: BucketConfig = BucketConfig::new(10, 5.0);

    /// AI calls: analyze_processes, analyze_context, ai_chat, validate_api_key.
    /// 10 calls burst, refills at 2/sec — interactive chat needs headroom.
    pub const AI: BucketConfig = BucketConfig::new(10, 2.0);

    /// Browser control: close_browser_tab, focus_browser_tab.
    /// 30 actions burst, refills at 10/sec — closing all tabs in a browser
    /// can easily exceed the old limit of 10.
    pub const BROWSER: BucketConfig = BucketConfig::new(30, 10.0);

    /// Cloud/config operations: save_ai_config, save_cloud_key.
    /// 5 operations burst, refills at 2/sec.
    pub const CONFIG: BucketConfig = BucketConfig::new(5, 2.0);
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
        eprintln!(
            "[rate-limit] REJECTED '{}' — tokens exhausted (capacity={}, refill={}/s)",
            bucket_name, bucket.capacity, bucket.refill_per_sec,
        );
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
    if let Ok(mut guard) = daily_limiters().lock() {
        guard.clear();
    }
}

// =============================================================================
// Daily limit (cost containment, not DDoS protection).
// =============================================================================

/// Fallback cap when no user-configured limit is available. Chosen to be
/// generous for interactive use (about 8 calls/hour continuous) yet bounded
/// enough to protect against a runaway loop burning tokens on a paid LLM
/// provider overnight.
pub const DEFAULT_AI_DAILY_LIMIT: u32 = 200;

/// Per-UTC-day call counter. Complements [`TokenBucket`]: the token bucket
/// prevents *bursts* (e.g. a loop hitting the API 100x in one second), and
/// the daily bucket caps *total cost per day*. Both fire independently.
///
/// The counter resets when the UTC day index advances. Persisting the
/// counter across app restarts is deliberately not done — a restart-evasion
/// attack would require the user to actively restart the app many times in
/// a day, which is both unlikely and already rate-limited by the token
/// bucket.
#[derive(Debug)]
pub struct DailyBucket {
    count: u32,
    limit: u32,
    day_index: u64,
}

impl DailyBucket {
    pub const fn new(limit: u32) -> Self {
        Self {
            count: 0,
            limit,
            day_index: 0,
        }
    }

    fn current_day_index() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() / 86_400)
            .unwrap_or(0)
    }

    /// Try to consume one call. Returns `true` if allowed, `false` if the
    /// daily limit has been reached. The counter resets when the UTC day
    /// index advances (so the first call after midnight UTC always
    /// succeeds if the limit is >= 1).
    pub fn try_acquire(&mut self, effective_limit: u32) -> bool {
        let today = Self::current_day_index();
        if today != self.day_index {
            self.day_index = today;
            self.count = 0;
        }
        self.limit = effective_limit;
        if self.count < self.limit {
            self.count += 1;
            true
        } else {
            false
        }
    }

    /// Returns `(used_today, configured_limit)`. If the day rolled over,
    /// `used_today` is reported as 0 even before the next `try_acquire`.
    pub fn snapshot(&self) -> (u32, u32) {
        let today = Self::current_day_index();
        let used = if today == self.day_index {
            self.count
        } else {
            0
        };
        (used, self.limit)
    }
}

static DAILY_LIMITERS: OnceLock<Mutex<HashMap<&'static str, DailyBucket>>> = OnceLock::new();

fn daily_limiters() -> &'static Mutex<HashMap<&'static str, DailyBucket>> {
    DAILY_LIMITERS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Check whether a command is within its daily budget.
///
/// `effective_limit` is the per-call cap from user settings (e.g. 200).
/// Passing 0 disables the daily check entirely (the limit becomes
/// unenforceable and every call is allowed), which lets the UI offer an
/// explicit "unlimited" mode for offline providers like Ollama.
pub fn check_daily_limit(bucket_name: &'static str, effective_limit: u32) -> Result<(), String> {
    if effective_limit == 0 {
        return Ok(());
    }
    let mut guard = daily_limiters()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let bucket = guard
        .entry(bucket_name)
        .or_insert_with(|| DailyBucket::new(effective_limit));
    if bucket.try_acquire(effective_limit) {
        Ok(())
    } else {
        let (used, limit) = bucket.snapshot();
        Err(format!(
            "Daily AI call limit reached ({used}/{limit}). Raise ai_daily_limit in settings or wait until UTC midnight to continue."
        ))
    }
}

/// Non-destructive read of a daily bucket, useful for UIs that want to
/// show "X / Y calls today".
pub fn daily_usage(bucket_name: &'static str) -> (u32, u32) {
    let guard = daily_limiters()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard
        .get(bucket_name)
        .map(|bucket| bucket.snapshot())
        .unwrap_or((0, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_BUCKET: BucketConfig = BucketConfig::new(3, 100.0);

    /// Serializes the burst-bucket tests: they all share the process-global
    /// limiter maps and call `reset_all()`, which would wipe buckets out
    /// from under parallel peers (e.g. a `reset_all()` landing between the
    /// two `check_rate_limit` calls of `refills_over_time` recreates the
    /// bucket at full capacity and fails the `is_err()` assertion).
    /// The daily-bucket tests below avoid this with unique bucket names.
    static BURST_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn allows_requests_within_capacity() {
        let _burst_guard = BURST_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();
        assert!(check_rate_limit("test_cap", &TEST_BUCKET).is_ok());
        assert!(check_rate_limit("test_cap", &TEST_BUCKET).is_ok());
        assert!(check_rate_limit("test_cap", &TEST_BUCKET).is_ok());
    }

    #[test]
    fn rejects_after_capacity_exhausted() {
        let _burst_guard = BURST_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
        let _burst_guard = BURST_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
        let _burst_guard = BURST_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
        let _burst_guard = BURST_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
        let _burst_guard = BURST_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();
        let config = BucketConfig::new(0, 0.0);
        let err = check_rate_limit("my_command", &config).unwrap_err();
        assert!(err.contains("my_command"));
    }

    // --- Daily bucket ---

    // NOTE on test isolation: these tests deliberately avoid calling
    // `reset_all()` because it is shared across every parallel test in
    // the file. Each daily-bucket test uses a unique bucket name instead,
    // which provides the same isolation without racing against peer tests
    // that also wipe the global map.

    #[test]
    fn daily_bucket_allows_up_to_limit() {
        assert!(check_daily_limit("daily_allows_up_to_limit", 3).is_ok());
        assert!(check_daily_limit("daily_allows_up_to_limit", 3).is_ok());
        assert!(check_daily_limit("daily_allows_up_to_limit", 3).is_ok());
        // 4th call in the same UTC day must be rejected.
        let err = check_daily_limit("daily_allows_up_to_limit", 3).unwrap_err();
        assert!(err.contains("Daily AI call limit reached"));
        assert!(err.contains("3/3"));
    }

    #[test]
    fn daily_bucket_zero_limit_means_unlimited() {
        // Useful for Ollama users: the daily cap is off because there is no
        // per-call cost. Burst bucket still protects against runaway loops.
        for _ in 0..100 {
            assert!(check_daily_limit("daily_zero_limit_unlimited", 0).is_ok());
        }
    }

    #[test]
    fn daily_usage_reports_counter_and_limit() {
        let _ = check_daily_limit("daily_usage_reports", 10);
        let _ = check_daily_limit("daily_usage_reports", 10);
        let (used, limit) = daily_usage("daily_usage_reports");
        assert_eq!(used, 2);
        assert_eq!(limit, 10);
    }

    #[test]
    fn daily_usage_of_unknown_bucket_is_zero() {
        let (used, limit) = daily_usage("daily_never_touched_sentinel");
        assert_eq!(used, 0);
        assert_eq!(limit, 0);
    }

    #[test]
    fn daily_bucket_tracks_day_rollover_logic() {
        // Directly exercise the struct to avoid depending on the real clock.
        let mut bucket = DailyBucket::new(2);
        bucket.day_index = DailyBucket::current_day_index();
        assert!(bucket.try_acquire(2));
        assert!(bucket.try_acquire(2));
        assert!(!bucket.try_acquire(2));
        // Simulate a day rollover — counter must reset.
        bucket.day_index = bucket.day_index.saturating_sub(1);
        assert!(bucket.try_acquire(2));
        let (used, limit) = bucket.snapshot();
        assert_eq!(used, 1);
        assert_eq!(limit, 2);
    }

    #[test]
    fn default_ai_daily_limit_is_non_trivial() {
        const {
            assert!(DEFAULT_AI_DAILY_LIMIT >= 50);
            assert!(DEFAULT_AI_DAILY_LIMIT <= 10_000);
        }
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
