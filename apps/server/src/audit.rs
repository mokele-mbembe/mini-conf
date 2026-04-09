use crate::error::ApiError;
use serde_json::Value;
use sqlx::{Executor, PgPool, Postgres};

#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    pub project_id: Option<i64>,
    pub user_id: Option<i64>,
    pub action: &'static str,
    pub resource_type: &'static str,
    pub resource_id: String,
    pub detail: Option<Value>,
}

pub async fn write_audit_log<'e, E>(executor: E, entry: AuditLogEntry) -> Result<(), ApiError>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(
        r#"
        INSERT INTO audit_logs (
            project_id,
            user_id,
            action,
            resource_type,
            resource_id,
            detail
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(entry.project_id)
    .bind(entry.user_id)
    .bind(entry.action)
    .bind(entry.resource_type)
    .bind(entry.resource_id)
    .bind(sanitize_audit_detail(entry.detail))
    .execute(executor)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(())
}

pub async fn write_audit_log_best_effort(pool: &PgPool, entry: AuditLogEntry) {
    let _ = write_audit_log(pool, entry).await;
}

pub fn sanitize_audit_detail(detail: Option<Value>) -> Option<Value> {
    detail.map(sanitize_value)
}

fn sanitize_value(value: Value) -> Value {
    match value {
        Value::Object(mut object) => {
            for key in [
                "content",
                "before_content",
                "after_content",
                "token",
                "password",
                "secret",
                "secret_paths",
                "draft_content",
                "release_content",
            ] {
                object.remove(key);
            }

            for child in object.values_mut() {
                let current = std::mem::take(child);
                *child = sanitize_value(current);
            }

            Value::Object(object)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sanitize_value).collect()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_audit_detail;

    #[test]
    fn strips_sensitive_fields_from_audit_detail() {
        let detail = sanitize_audit_detail(Some(serde_json::json!({
            "content": "plain",
            "token": "mc_live_secret",
            "token_preview": "mc_live_***",
            "nested": {
                "after_content": "still secret",
                "role": "admin"
            }
        })))
        .expect("detail should remain present");

        assert_eq!(
            detail,
            serde_json::json!({
                "token_preview": "mc_live_***",
                "nested": {
                    "role": "admin"
                }
            })
        );
    }
}
