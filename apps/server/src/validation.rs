use crate::error::ApiError;
use serde_json::Value as JsonValue;

const REDACTED_PLACEHOLDER: &str = "***REDACTED***";

pub struct RedactedContent {
    pub content: String,
    pub redacted: bool,
}

pub fn validate_content(format: &str, content: &str) -> Result<(), ApiError> {
    parse_document(format, content).map(|_| ())
}

pub fn redact_content(
    format: &str,
    content: &str,
    sensitivity: &str,
    secret_paths: Option<&[String]>,
) -> RedactedContent {
    if sensitivity != "secret" {
        return RedactedContent {
            content: content.to_owned(),
            redacted: false,
        };
    }

    let Some(paths) = secret_paths.filter(|paths| !paths.is_empty()) else {
        return fully_redacted(format);
    };

    let Ok(mut document) = parse_document(format, content) else {
        return fully_redacted(format);
    };

    let mut any_path_redacted = false;
    for path in paths {
        if redact_path(&mut document, path) {
            any_path_redacted = true;
        }
    }

    if !any_path_redacted {
        return fully_redacted(format);
    }

    match serialize_document(format, &document) {
        Ok(content) => RedactedContent {
            content,
            redacted: true,
        },
        Err(_) => fully_redacted(format),
    }
}

pub fn parse_document(format: &str, content: &str) -> Result<JsonValue, ApiError> {
    match normalize_format(format) {
        "json" => serde_json::from_str(content).map_err(|_| {
            ApiError::unprocessable_entity(
                "draft_validation_failed",
                "draft content is not valid json",
            )
        }),
        "yaml" | "yml" => serde_yaml::from_str::<serde_yaml::Value>(content)
            .map_err(|_| {
                ApiError::unprocessable_entity(
                    "draft_validation_failed",
                    "draft content is not valid yaml",
                )
            })
            .and_then(yaml_to_json),
        "toml" => toml::from_str::<toml::Value>(content)
            .map_err(|_| {
                ApiError::unprocessable_entity(
                    "draft_validation_failed",
                    "draft content is not valid toml",
                )
            })
            .and_then(toml_to_json),
        _ => Err(ApiError::unprocessable_entity(
            "draft_validation_failed",
            "unsupported config file format",
        )),
    }
}

fn serialize_document(format: &str, document: &JsonValue) -> Result<String, ApiError> {
    match normalize_format(format) {
        "json" => serde_json::to_string_pretty(document).map_err(|_| ApiError::internal()),
        "yaml" | "yml" => serde_yaml::to_string(document).map_err(|_| ApiError::internal()),
        "toml" => json_to_toml(document)
            .and_then(|value| toml::to_string_pretty(&value).map_err(|_| ApiError::internal())),
        _ => Err(ApiError::internal()),
    }
}

fn normalize_format(format: &str) -> &str {
    match format.trim().to_ascii_lowercase().as_str() {
        "yaml" => "yaml",
        "yml" => "yml",
        "json" => "json",
        "toml" => "toml",
        _ => format,
    }
}

fn fully_redacted(format: &str) -> RedactedContent {
    let content = match normalize_format(format) {
        "json" => serde_json::json!({ "redacted": REDACTED_PLACEHOLDER }).to_string(),
        "toml" => format!("redacted = \"{REDACTED_PLACEHOLDER}\"\n"),
        _ => format!("redacted: {REDACTED_PLACEHOLDER}\n"),
    };

    RedactedContent {
        content,
        redacted: true,
    }
}

fn redact_path(document: &mut JsonValue, path: &str) -> bool {
    let Some(mut segments) = parse_secret_path(path) else {
        return false;
    };

    redact_segments(document, &mut segments)
}

fn redact_segments(document: &mut JsonValue, segments: &mut Vec<String>) -> bool {
    if segments.is_empty() {
        *document = JsonValue::String(REDACTED_PLACEHOLDER.to_owned());
        return true;
    }

    let segment = segments.remove(0);
    match document {
        JsonValue::Object(map) => map
            .get_mut(&segment)
            .is_some_and(|child| redact_segments(child, segments)),
        JsonValue::Array(items) => segment
            .parse::<usize>()
            .ok()
            .and_then(|index| items.get_mut(index))
            .is_some_and(|child| redact_segments(child, segments)),
        _ => false,
    }
}

fn parse_secret_path(path: &str) -> Option<Vec<String>> {
    let trimmed = path.trim();
    let path = trimmed.strip_prefix("$.")?;
    let segments: Vec<String> = path
        .split('.')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    if segments.is_empty() {
        None
    } else {
        Some(segments)
    }
}

fn yaml_to_json(value: serde_yaml::Value) -> Result<JsonValue, ApiError> {
    serde_json::to_value(value).map_err(|_| {
        ApiError::unprocessable_entity(
            "draft_validation_failed",
            "yaml content contains unsupported value types",
        )
    })
}

fn toml_to_json(value: toml::Value) -> Result<JsonValue, ApiError> {
    serde_json::to_value(value).map_err(|_| {
        ApiError::unprocessable_entity(
            "draft_validation_failed",
            "toml content contains unsupported value types",
        )
    })
}

fn json_to_toml(value: &JsonValue) -> Result<toml::Value, ApiError> {
    toml::Value::try_from(value.clone()).map_err(|_| ApiError::internal())
}

#[cfg(test)]
mod tests {
    use super::{REDACTED_PLACEHOLDER, redact_content, validate_content};

    #[test]
    fn validates_basic_yaml_content() {
        assert!(validate_content("yaml", "log_level: info\n").is_ok());
    }

    #[test]
    fn rejects_invalid_yaml_content() {
        let error = validate_content("yaml", "log_level: [\n").err();

        assert!(error.is_some());
        assert_eq!(
            error.map(|error| error.into_body().message),
            Some("draft content is not valid yaml".to_owned())
        );
    }

    #[test]
    fn redacts_yaml_secret_paths() {
        let result = redact_content(
            "yaml",
            "wifi:\n  password: secret\n",
            "secret",
            Some(&["$.wifi.password".to_owned()]),
        );

        assert!(result.redacted);
        assert!(result.content.contains(REDACTED_PLACEHOLDER));
        assert!(!result.content.contains("secret"));
    }

    #[test]
    fn validates_basic_toml_content() {
        assert!(validate_content("toml", "log_level = \"info\"\n").is_ok());
    }

    #[test]
    fn rejects_invalid_toml_content() {
        let error = validate_content("toml", "log_level = ").err();

        assert!(error.is_some());
        assert_eq!(
            error.map(|error| error.into_body().message),
            Some("draft content is not valid toml".to_owned())
        );
    }

    #[test]
    fn redacts_toml_secret_paths() {
        let result = redact_content(
            "toml",
            "[wifi]\npassword = \"secret\"\n",
            "secret",
            Some(&["$.wifi.password".to_owned()]),
        );

        assert!(result.redacted);
        assert!(result.content.contains(REDACTED_PLACEHOLDER));
        assert!(!result.content.contains("secret"));
    }
}
