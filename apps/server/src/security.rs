use crate::error::ApiError;
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const FAILURE_WINDOW: Duration = Duration::from_secs(10 * 60);
const BLOCK_WINDOW: Duration = Duration::from_secs(15 * 60);
const MAX_FAILURES: usize = 5;

const SECURITY_HEADERS: [(&str, &str); 5] = [
    ("x-content-type-options", "nosniff"),
    ("x-frame-options", "DENY"),
    ("referrer-policy", "no-referrer"),
    (
        "permissions-policy",
        "camera=(), microphone=(), geolocation=()",
    ),
    ("cross-origin-opener-policy", "same-origin"),
];

#[derive(Debug, Default)]
pub struct LoginThrottle {
    entries: Mutex<HashMap<String, LoginThrottleEntry>>,
}

#[derive(Debug, Default)]
struct LoginThrottleEntry {
    failures: VecDeque<Instant>,
    blocked_until: Option<Instant>,
}

impl LoginThrottle {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn ensure_allowed(&self, key: &str) -> Result<(), ApiError> {
        let mut entries = self.entries.lock().map_err(|_| ApiError::internal())?;
        let now = Instant::now();

        if let Some(entry) = entries.get_mut(key) {
            prune_failures(entry, now);

            if let Some(blocked_until) = entry.blocked_until {
                if blocked_until > now {
                    return Err(ApiError::too_many_requests(
                        "auth_rate_limited",
                        "Too many failed login attempts; try again later",
                    ));
                }

                entry.blocked_until = None;
            }

            if entry.failures.is_empty() && entry.blocked_until.is_none() {
                entries.remove(key);
            }
        }

        Ok(())
    }

    pub fn record_failure(&self, key: &str) {
        if let Ok(mut entries) = self.entries.lock() {
            let now = Instant::now();
            let entry = entries.entry(key.to_owned()).or_default();
            prune_failures(entry, now);
            entry.failures.push_back(now);

            if entry.failures.len() >= MAX_FAILURES {
                entry.failures.clear();
                entry.blocked_until = Some(now + BLOCK_WINDOW);
            }
        }
    }

    pub fn record_success(&self, key: &str) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.remove(key);
        }
    }
}

pub fn login_throttle_key(headers: &HeaderMap, username: &str) -> String {
    let client = client_ip(headers).unwrap_or("unknown");
    format!("{}|{}", username.trim().to_ascii_lowercase(), client)
}

pub fn apply_security_headers(headers: &mut HeaderMap) {
    for (name, value) in SECURITY_HEADERS {
        headers.insert(
            HeaderName::from_static(name),
            HeaderValue::from_static(value),
        );
    }
}

fn prune_failures(entry: &mut LoginThrottleEntry, now: Instant) {
    while let Some(failed_at) = entry.failures.front() {
        if now.duration_since(*failed_at) <= FAILURE_WINDOW {
            break;
        }

        entry.failures.pop_front();
    }
}

fn client_ip(headers: &HeaderMap) -> Option<&str> {
    let forwarded = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty());

    forwarded.or_else(|| {
        headers
            .get("x-real-ip")
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
    })
}

#[cfg(test)]
mod tests {
    use super::{LoginThrottle, apply_security_headers, login_throttle_key};
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn login_throttle_key_uses_forwarded_client_ip() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.9, 203.0.113.10"),
        );

        assert_eq!(
            login_throttle_key(&headers, " Admin "),
            "admin|203.0.113.9".to_owned()
        );
    }

    #[test]
    fn security_headers_are_applied() {
        let mut headers = HeaderMap::new();

        apply_security_headers(&mut headers);

        assert_eq!(
            headers.get("x-content-type-options"),
            Some(&HeaderValue::from_static("nosniff"))
        );
        assert_eq!(
            headers.get("x-frame-options"),
            Some(&HeaderValue::from_static("DENY"))
        );
    }

    #[test]
    fn login_throttle_blocks_after_too_many_failures() {
        let throttle = LoginThrottle::default();
        let key = "admin|unknown";

        for _ in 0..5 {
            throttle.record_failure(key);
        }

        assert_eq!(
            throttle
                .ensure_allowed(key)
                .map_err(|error| error.into_body().code),
            Err("auth_rate_limited".to_owned())
        );
    }

    #[test]
    fn login_throttle_clears_failures_after_success() {
        let throttle = LoginThrottle::default();
        let key = "admin|unknown";

        throttle.record_failure(key);
        throttle.record_success(key);

        assert!(throttle.ensure_allowed(key).is_ok());
    }
}
