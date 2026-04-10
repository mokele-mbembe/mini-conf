use crate::error::ApiError;
use serde_json::{Map as JsonMap, Value as JsonValue};

const REDACTED_PLACEHOLDER: &str = "***REDACTED***";
type SchemaValidator = fn(&JsonValue) -> Result<(), ApiError>;

pub struct ValidatorRegistry;

pub struct RedactedContent {
    pub content: String,
    pub redacted: bool,
}

impl ValidatorRegistry {
    pub fn validate_content(
        format: &str,
        content: &str,
        schema_name: Option<&str>,
        schema_version: Option<&str>,
    ) -> Result<(), ApiError> {
        let document = parse_document(format, content)?;

        if let Some(validator) = lookup_validator(schema_name, schema_version) {
            validator(&document)?;
        }

        Ok(())
    }
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
        _ => Err(ApiError::internal()),
    }
}

fn normalize_format(format: &str) -> &str {
    match format.trim().to_ascii_lowercase().as_str() {
        "yaml" => "yaml",
        "yml" => "yml",
        "json" => "json",
        _ => format,
    }
}

fn lookup_validator(
    schema_name: Option<&str>,
    schema_version: Option<&str>,
) -> Option<SchemaValidator> {
    match (schema_name, schema_version) {
        (Some("coffee-main"), Some("v1" | "v2")) => Some(validate_main_config),
        (Some("alpha-main"), Some("v1")) => Some(validate_main_config),
        _ => None,
    }
}

fn validate_main_config(document: &JsonValue) -> Result<(), ApiError> {
    let object = document.as_object().ok_or_else(|| {
        ApiError::unprocessable_entity(
            "draft_validation_failed",
            "schema validation requires a top-level object",
        )
    })?;

    validate_positive_integer_field(object, "poll_interval_ms")?;
    validate_positive_integer_field(object, "poll_interval_sec")?;

    if let Some(value) = object.get("log_level")
        && !value.is_string()
    {
        return Err(ApiError::unprocessable_entity(
            "draft_validation_failed",
            "log_level must be a string",
        ));
    }

    Ok(())
}

fn validate_positive_integer_field(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<(), ApiError> {
    let Some(value) = object.get(field) else {
        return Ok(());
    };

    let Some(number) = value.as_i64() else {
        return Err(ApiError::unprocessable_entity(
            "draft_validation_failed",
            match field {
                "poll_interval_ms" => "poll_interval_ms must be an integer",
                "poll_interval_sec" => "poll_interval_sec must be an integer",
                _ => "field must be an integer",
            },
        ));
    };

    if number <= 0 {
        return Err(ApiError::unprocessable_entity(
            "draft_validation_failed",
            match field {
                "poll_interval_ms" => "poll_interval_ms must be greater than zero",
                "poll_interval_sec" => "poll_interval_sec must be greater than zero",
                _ => "field must be greater than zero",
            },
        ));
    }

    Ok(())
}

fn fully_redacted(format: &str) -> RedactedContent {
    let content = match normalize_format(format) {
        "json" => serde_json::json!({ "redacted": REDACTED_PLACEHOLDER }).to_string(),
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

#[cfg(test)]
mod tests {
    use super::{REDACTED_PLACEHOLDER, ValidatorRegistry, redact_content};

    #[test]
    fn validates_known_schema() {
        assert!(
            ValidatorRegistry::validate_content(
                "yaml",
                "poll_interval_ms: 5000\n",
                Some("coffee-main"),
                Some("v1"),
            )
            .is_ok()
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
}
