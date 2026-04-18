use crate::constants::{
    SECURITY_RATE_LIMIT_DEFAULT_REQUESTS, SECURITY_RATE_LIMIT_DEFAULT_WINDOW_SECONDS,
};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_requests: u64,
    pub window_seconds: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: SECURITY_RATE_LIMIT_DEFAULT_REQUESTS,
            window_seconds: SECURITY_RATE_LIMIT_DEFAULT_WINDOW_SECONDS,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub limit: u64,
    pub remaining: u64,
    pub reset_in_seconds: u64,
}

#[derive(Debug)]
struct RequestWindow {
    requests: Vec<Instant>,
    window_start: Instant,
}

impl RequestWindow {
    fn new() -> Self {
        Self {
            requests: Vec::new(),
            window_start: Instant::now(),
        }
    }

    fn cleanup(&mut self, window_duration: Duration) {
        let now = Instant::now();
        let cutoff = now - window_duration;

        self.requests.retain(|&t| t > cutoff);

        if !self.requests.is_empty() {
            let earliest = *self.requests.first().unwrap();
            if earliest < self.window_start {
                self.window_start = earliest;
            }
        } else {
            self.window_start = now;
        }
    }

    fn add_request(&mut self) {
        self.requests.push(Instant::now());
    }

    fn count(&self) -> usize {
        self.requests.len()
    }

    fn reset_time(&self, window_duration: Duration) -> Duration {
        let now = Instant::now();
        let window_end = self.window_start + window_duration;
        if window_end > now {
            window_end - now
        } else {
            Duration::from_secs(0)
        }
    }
}

pub struct RateLimiter {
    config: RateLimitConfig,
    ip_windows: Arc<DashMap<String, RequestWindow>>,
    user_windows: Arc<DashMap<String, RequestWindow>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::with_config(RateLimitConfig::default())
    }

    pub fn with_config(config: RateLimitConfig) -> Self {
        Self {
            config,
            ip_windows: Arc::new(DashMap::new()),
            user_windows: Arc::new(DashMap::new()),
        }
    }

    pub fn check_ip(&self, ip: &str) -> RateLimitResult {
        self.check(&self.ip_windows, ip)
    }

    pub fn check_user(&self, user_id: &str) -> RateLimitResult {
        self.check(&self.user_windows, user_id)
    }

    fn check(&self, windows: &DashMap<String, RequestWindow>, key: &str) -> RateLimitResult {
        let window_duration = Duration::from_secs(self.config.window_seconds);
        let max_requests = self.config.max_requests;

        let mut window = windows
            .entry(key.to_string())
            .or_insert_with(RequestWindow::new);

        window.cleanup(window_duration);

        let current_count = window.count() as u64;
        let allowed = current_count < max_requests;

        if allowed {
            window.add_request();
        }

        let remaining = if allowed {
            max_requests - (current_count + 1)
        } else {
            0
        };

        let reset_in_seconds = window.reset_time(window_duration).as_secs();

        RateLimitResult {
            allowed,
            limit: max_requests,
            remaining,
            reset_in_seconds,
        }
    }

    pub fn reset_ip(&self, ip: &str) {
        self.ip_windows.remove(ip);
    }

    pub fn reset_user(&self, user_id: &str) {
        self.user_windows.remove(user_id);
    }

    pub fn reset_all(&self) {
        self.ip_windows.clear();
        self.user_windows.clear();
    }

    pub fn get_ip_count(&self, ip: &str) -> u64 {
        self.ip_windows
            .get(ip)
            .map(|w| w.count() as u64)
            .unwrap_or(0)
    }

    pub fn get_user_count(&self, user_id: &str) -> u64 {
        self.user_windows
            .get(user_id)
            .map(|w| w.count() as u64)
            .unwrap_or(0)
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_requests, SECURITY_RATE_LIMIT_DEFAULT_REQUESTS);
        assert_eq!(
            config.window_seconds,
            SECURITY_RATE_LIMIT_DEFAULT_WINDOW_SECONDS
        );
    }

    #[test]
    fn test_rate_limiter_allows_requests_under_limit() {
        let config = RateLimitConfig {
            max_requests: 5,
            window_seconds: 60,
        };
        let limiter = RateLimiter::with_config(config);

        for i in 0..5 {
            let result = limiter.check_ip("127.0.0.1");
            assert!(result.allowed, "Request {} should be allowed", i);
            assert_eq!(result.limit, 5);
            assert_eq!(result.remaining, 4 - i);
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let config = RateLimitConfig {
            max_requests: 3,
            window_seconds: 60,
        };
        let limiter = RateLimiter::with_config(config);

        for _ in 0..3 {
            limiter.check_ip("192.168.1.1");
        }

        let result = limiter.check_ip("192.168.1.1");
        assert!(!result.allowed);
        assert_eq!(result.remaining, 0);
    }

    #[test]
    fn test_rate_limiter_different_ips_independent() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_seconds: 60,
        };
        let limiter = RateLimiter::with_config(config);

        limiter.check_ip("10.0.0.1");
        limiter.check_ip("10.0.0.1");
        let result1 = limiter.check_ip("10.0.0.1");
        assert!(!result1.allowed);

        let result2 = limiter.check_ip("10.0.0.2");
        assert!(result2.allowed);
    }

    #[test]
    fn test_rate_limiter_user_level() {
        let config = RateLimitConfig {
            max_requests: 10,
            window_seconds: 60,
        };
        let limiter = RateLimiter::with_config(config);

        for i in 0..10 {
            let result = limiter.check_user("user-123");
            assert!(result.allowed, "Request {} should be allowed", i);
        }

        let result = limiter.check_user("user-123");
        assert!(!result.allowed);
    }

    #[test]
    fn test_rate_limiter_reset_ip() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_seconds: 60,
        };
        let limiter = RateLimiter::with_config(config);

        limiter.check_ip("1.1.1.1");
        limiter.check_ip("1.1.1.1");
        assert!(!limiter.check_ip("1.1.1.1").allowed);

        limiter.reset_ip("1.1.1.1");
        assert!(limiter.check_ip("1.1.1.1").allowed);
    }

    #[test]
    fn test_rate_limiter_reset_all() {
        let config = RateLimitConfig {
            max_requests: 1,
            window_seconds: 60,
        };
        let limiter = RateLimiter::with_config(config);

        limiter.check_ip("2.2.2.2");
        limiter.check_user("user-a");
        assert!(!limiter.check_ip("2.2.2.2").allowed);
        assert!(!limiter.check_user("user-a").allowed);

        limiter.reset_all();
        assert!(limiter.check_ip("2.2.2.2").allowed);
        assert!(limiter.check_user("user-a").allowed);
    }

    #[tokio::test]
    async fn test_concurrent_rate_limiting() {
        let config = RateLimitConfig {
            max_requests: 100,
            window_seconds: 60,
        };
        let limiter = Arc::new(RateLimiter::with_config(config));

        let mut handles = Vec::new();
        for _ in 0..100 {
            let limiter_clone = limiter.clone();
            handles.push(tokio::spawn(async move {
                limiter_clone.check_ip("concurrent-test")
            }));
        }

        let results: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .collect();

        let allowed_count = results
            .iter()
            .filter(|r| r.as_ref().map(|res| res.allowed).unwrap_or(false))
            .count();

        assert_eq!(allowed_count, 100);

        let final_result = limiter.check_ip("concurrent-test");
        assert!(!final_result.allowed);
    }
}
