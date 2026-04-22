use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static UNIQUE_SCHEMA_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn test_database_url(test_name: &str) -> Option<String> {
    if let Ok(value) = std::env::var("TEST_DATABASE_URL") {
        if !value.trim().is_empty() {
            return Some(value);
        }
    }

    eprintln!("skipping {test_name} integration test: TEST_DATABASE_URL is not set");
    None
}

pub fn unique_schema_name(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let counter = UNIQUE_SCHEMA_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();

    format!("{prefix}_{pid}_{nanos}_{counter}")
}

pub fn with_search_path(database_url: &str, schema: &str) -> String {
    let separator = if database_url.contains('?') { '&' } else { '?' };
    format!("{database_url}{separator}options[search_path]={schema}")
}
