use std::time::{SystemTime, UNIX_EPOCH};

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

    format!("{prefix}_{nanos}")
}

pub fn with_search_path(database_url: &str, schema: &str) -> String {
    let separator = if database_url.contains('?') { '&' } else { '?' };
    format!("{database_url}{separator}options[search_path]={schema}")
}
