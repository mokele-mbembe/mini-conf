use crate::error::ApiError;
use axum::http::{HeaderMap, HeaderName, HeaderValue, header};
use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
    time::{Duration, Instant},
};

const FAILURE_WINDOW: Duration = Duration::from_secs(10 * 60);
const BLOCK_WINDOW: Duration = Duration::from_secs(15 * 60);
const MAX_FAILURES: usize = 5;

pub const OPEN_API_RATE_LIMIT: usize = 60;
pub const OPEN_API_RATE_WINDOW_SECS: u64 = 60;

const OPEN_API_RATE_WINDOW: Duration = Duration::from_secs(OPEN_API_RATE_WINDOW_SECS);

const SECURITY_HEADERS: [(&str, &str); 7] = [
    ("x-content-type-options", "nosniff"),
    ("x-frame-options", "DENY"),
    ("referrer-policy", "no-referrer"),
    (
        "permissions-policy",
        "camera=(), microphone=(), geolocation=()",
    ),
    ("cross-origin-opener-policy", "same-origin"),
    ("cross-origin-resource-policy", "same-origin"),
    ("x-permitted-cross-domain-policies", "none"),
];

const STRICT_TRANSPORT_SECURITY: (&str, &str) = (
    "strict-transport-security",
    "max-age=31536000; includeSubDomains",
);
const CONTENT_SECURITY_POLICY: &str = "content-security-policy";
const CONTENT_SECURITY_POLICY_BASE: &str = "default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'";

#[derive(Debug, Default)]
pub struct LoginThrottle {
    entries: Mutex<HashMap<String, LoginThrottleEntry>>,
}

#[derive(Debug, Default)]
pub struct OpenApiRateLimiter {
    entries: Mutex<HashMap<String, OpenApiRateLimitEntry>>,
}

#[derive(Debug, Default)]
struct LoginThrottleEntry {
    failures: VecDeque<Instant>,
    blocked_until: Option<Instant>,
}

#[derive(Debug, Default)]
struct OpenApiRateLimitEntry {
    requests: VecDeque<Instant>,
}

impl LoginThrottle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ensure_allowed(&self, key: &str) -> Result<(), ApiError> {
        let mut entries = self.entries.lock().map_err(|error| {
            ApiError::internal_with(error, "failed to lock login throttle entries")
        })?;
        let now = Instant::now();

        if let Some(entry) = entries.get_mut(key) {
            prune_login_failures(entry, now);

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
            prune_login_failures(entry, now);
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

impl OpenApiRateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ensure_request_allowed(&self, key: &str) -> Result<(), ApiError> {
        self.ensure_request_allowed_for_keys([key])
    }

    pub fn ensure_request_allowed_for_keys<I, K>(&self, keys: I) -> Result<(), ApiError>
    where
        I: IntoIterator<Item = K>,
        K: AsRef<str>,
    {
        let keys: Vec<String> = keys
            .into_iter()
            .map(|key| key.as_ref().to_owned())
            .collect();
        let mut entries = self.entries.lock().map_err(|error| {
            ApiError::internal_with(error, "failed to lock open api rate limit entries")
        })?;
        let now = Instant::now();

        for key in &keys {
            let entry = entries.entry(key.clone()).or_default();
            prune_open_api_requests(entry, now);

            if entry.requests.len() >= OPEN_API_RATE_LIMIT {
                return Err(ApiError::too_many_requests(
                    "open_api_rate_limited",
                    "Too many Open API requests; try again later",
                ));
            }
        }

        for key in keys {
            entries.entry(key).or_default().requests.push_back(now);
        }

        Ok(())
    }
}

pub fn login_throttle_key(headers: &HeaderMap, username: &str) -> String {
    let client = client_ip(headers).unwrap_or("unknown");
    format!("{}|{}", username.trim().to_ascii_lowercase(), client)
}

pub fn open_api_rate_limit_keys(headers: &HeaderMap) -> Vec<String> {
    let client = request_client_ip(headers);
    let mut keys = vec![format!("ip:{client}")];

    match bearer_token_fingerprint(headers) {
        Some(fingerprint) => keys.push(format!("token:{fingerprint}")),
        None => keys.push(format!("anonymous:{client}")),
    }

    keys
}

pub fn request_client_ip(headers: &HeaderMap) -> String {
    client_ip(headers).unwrap_or("unknown").to_owned()
}

pub fn has_bearer_token(headers: &HeaderMap) -> bool {
    bearer_token(headers).is_some()
}

pub fn apply_security_headers(
    headers: &mut HeaderMap,
    include_hsts: bool,
    csp_connect_src_extra: &[String],
) {
    for (name, value) in SECURITY_HEADERS {
        headers.insert(
            HeaderName::from_static(name),
            HeaderValue::from_static(value),
        );
    }

    let policy = content_security_policy(csp_connect_src_extra);
    match HeaderValue::from_str(&policy) {
        Ok(value) => {
            headers.insert(HeaderName::from_static(CONTENT_SECURITY_POLICY), value);
        }
        Err(error) => {
            tracing::error!(?error, "failed to build content security policy header");
        }
    }

    if include_hsts {
        headers.insert(
            HeaderName::from_static(STRICT_TRANSPORT_SECURITY.0),
            HeaderValue::from_static(STRICT_TRANSPORT_SECURITY.1),
        );
    }
}

fn content_security_policy(csp_connect_src_extra: &[String]) -> String {
    if csp_connect_src_extra.is_empty() {
        return CONTENT_SECURITY_POLICY_BASE.to_owned();
    }

    format!(
        "{} {}",
        CONTENT_SECURITY_POLICY_BASE,
        csp_connect_src_extra.join(" ")
    )
}

fn prune_login_failures(entry: &mut LoginThrottleEntry, now: Instant) {
    while let Some(failed_at) = entry.failures.front() {
        if now.duration_since(*failed_at) <= FAILURE_WINDOW {
            break;
        }

        entry.failures.pop_front();
    }
}

fn prune_open_api_requests(entry: &mut OpenApiRateLimitEntry, now: Instant) {
    while let Some(requested_at) = entry.requests.front() {
        if now.duration_since(*requested_at) <= OPEN_API_RATE_WINDOW {
            break;
        }

        entry.requests.pop_front();
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

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())?;

    let token = value.strip_prefix("Bearer ")?;
    let token = token.trim();

    if token.is_empty() { None } else { Some(token) }
}

fn bearer_token_fingerprint(headers: &HeaderMap) -> Option<String> {
    let token = bearer_token(headers)?;
    let hash = crate::auth::hash_bearer_token(token);

    Some(hash.chars().take(16).collect())
}

#[cfg(test)]
mod tests {
    use super::{
        LoginThrottle, OPEN_API_RATE_LIMIT, OpenApiRateLimiter, apply_security_headers,
        has_bearer_token, login_throttle_key, open_api_rate_limit_keys, request_client_ip,
    };
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

        apply_security_headers(&mut headers, false, &[]);

        assert_eq!(
            headers.get("x-content-type-options"),
            Some(&HeaderValue::from_static("nosniff"))
        );
        assert_eq!(
            headers.get("x-frame-options"),
            Some(&HeaderValue::from_static("DENY"))
        );
        assert_eq!(
            headers.get("content-security-policy"),
            Some(&HeaderValue::from_static(
                "default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'"
            ))
        );
        assert!(headers.get("strict-transport-security").is_none());
    }

    #[test]
    fn security_headers_include_configured_connect_sources() {
        let mut headers = HeaderMap::new();

        apply_security_headers(&mut headers, false, &["https://api.example.com".to_owned()]);

        assert_eq!(
            headers.get("content-security-policy"),
            Some(&HeaderValue::from_static(
                "default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self' https://api.example.com"
            ))
        );
    }

    #[test]
    fn hsts_header_is_optional() {
        let mut headers = HeaderMap::new();

        apply_security_headers(&mut headers, true, &[]);

        assert_eq!(
            headers.get("strict-transport-security"),
            Some(&HeaderValue::from_static(
                "max-age=31536000; includeSubDomains"
            ))
        );
    }

    #[test]
    fn open_api_key_uses_client_ip_and_token_fingerprint() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", HeaderValue::from_static("198.51.100.20"));
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer mini-conf-token"),
        );

        let keys = open_api_rate_limit_keys(&headers);

        assert!(keys.contains(&"ip:198.51.100.20".to_owned()));
        assert!(keys.iter().any(|key| key.starts_with("token:")));
        assert!(has_bearer_token(&headers));
        assert_eq!(request_client_ip(&headers), "198.51.100.20");
    }

    #[test]
    fn open_api_key_supports_anonymous_requests() {
        let headers = HeaderMap::new();

        assert_eq!(
            open_api_rate_limit_keys(&headers),
            vec!["ip:unknown".to_owned(), "anonymous:unknown".to_owned()]
        );
        assert!(!has_bearer_token(&headers));
    }

    #[test]
    fn open_api_key_treats_invalid_bearer_headers_as_anonymous() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", HeaderValue::from_static("192.0.2.30"));
        headers.insert("authorization", HeaderValue::from_static("Basic abc"));

        assert_eq!(
            open_api_rate_limit_keys(&headers),
            vec![
                "ip:192.0.2.30".to_owned(),
                "anonymous:192.0.2.30".to_owned()
            ]
        );
        assert!(!has_bearer_token(&headers));

        headers.insert("authorization", HeaderValue::from_static("Bearer   "));

        assert_eq!(
            open_api_rate_limit_keys(&headers),
            vec![
                "ip:192.0.2.30".to_owned(),
                "anonymous:192.0.2.30".to_owned()
            ]
        );
        assert!(!has_bearer_token(&headers));
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

    #[test]
    fn open_api_rate_limiter_blocks_after_threshold() {
        let limiter = OpenApiRateLimiter::default();
        let key = "unknown|anonymous";

        for _ in 0..OPEN_API_RATE_LIMIT {
            assert!(limiter.ensure_request_allowed(key).is_ok());
        }

        assert_eq!(
            limiter
                .ensure_request_allowed(key)
                .map_err(|error| error.into_body().code),
            Err("open_api_rate_limited".to_owned())
        );
    }

    #[test]
    fn open_api_rate_limiter_blocks_when_any_bucket_is_full() {
        let limiter = OpenApiRateLimiter::default();
        let ip_key = "ip:203.0.113.20".to_owned();

        for index in 0..OPEN_API_RATE_LIMIT {
            assert!(
                limiter
                    .ensure_request_allowed_for_keys(&[
                        ip_key.clone(),
                        format!("token:fingerprint-{index}")
                    ])
                    .is_ok()
            );
        }

        assert_eq!(
            limiter
                .ensure_request_allowed_for_keys(&[ip_key, "token:fingerprint-new".to_owned(),])
                .map_err(|error| error.into_body().code),
            Err("open_api_rate_limited".to_owned())
        );
    }

    #[test]
    fn open_api_rate_limiter_records_all_keys_for_one_request() {
        let limiter = OpenApiRateLimiter::default();
        let token_key = "token:shared".to_owned();

        for index in 0..OPEN_API_RATE_LIMIT {
            assert!(
                limiter
                    .ensure_request_allowed_for_keys(&[
                        format!("ip:198.51.100.{index}"),
                        token_key.clone(),
                    ])
                    .is_ok()
            );
        }

        assert_eq!(
            limiter
                .ensure_request_allowed_for_keys(&["ip:198.51.100.200".to_owned(), token_key,])
                .map_err(|error| error.into_body().code),
            Err("open_api_rate_limited".to_owned())
        );
    }
}
