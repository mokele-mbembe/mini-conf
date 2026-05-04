use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use infra::testing::{test_database_url, unique_schema_name, with_search_path};
use server::{bootstrap, config::AppConfig, error::ErrorResponse};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use tower::util::ServiceExt;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

async fn setup_app() -> TestResult<Option<(axum::Router, PgPool, String, String)>> {
    let Some(database_url) = test_database_url("auth session") else {
        return Ok(None);
    };
    let schema = unique_schema_name("mini_conf_auth_session");
    let mut admin = PgConnection::connect(&database_url).await?;
    admin
        .execute(format!("CREATE SCHEMA {schema}").as_str())
        .await?;

    let state = bootstrap::build_state(AppConfig {
        init_db_on_boot: true,
        init_admin_username: Some("admin".to_owned()),
        init_admin_password: Some("admin123456".to_owned()),
        database_url: with_search_path(&database_url, &schema),
        ..AppConfig::default()
    })
    .await?;

    let pool = state
        .db_pool()
        .cloned()
        .ok_or_else(|| std::io::Error::other("db pool should be present after bootstrap"))?;
    sqlx::query(
        r#"
        UPDATE system_settings
        SET
            setup_completed_at = COALESCE(setup_completed_at, NOW()),
            updated_at = NOW()
        WHERE id = 1
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(Some((server::app(state), pool, database_url, schema)))
}

async fn teardown(database_url: &str, schema: &str, pool: PgPool) -> TestResult {
    pool.close().await;

    let mut admin = PgConnection::connect(database_url).await?;
    admin
        .execute(format!("DROP SCHEMA IF EXISTS {schema} CASCADE").as_str())
        .await?;
    Ok(())
}

async fn read_json<T: serde::de::DeserializeOwned>(
    response: axum::response::Response,
) -> TestResult<T> {
    let body = to_bytes(response.into_body(), usize::MAX).await?;
    Ok(serde_json::from_slice(&body)?)
}

fn session_cookie(response: &axum::response::Response) -> TestResult<String> {
    response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::to_owned)
        .ok_or_else(|| std::io::Error::other("set-cookie should contain a session cookie").into())
}

fn auth_cookie_parts(response: &axum::response::Response) -> TestResult<(String, String, String)> {
    let session = session_cookie(response)?;
    let csrf_cookie = cookie_value(response, "mini_conf_csrf")?;
    let csrf_token = csrf_cookie
        .strip_prefix("mini_conf_csrf=")
        .ok_or_else(|| std::io::Error::other("csrf cookie should have expected prefix"))?
        .to_owned();

    Ok((session, csrf_cookie, csrf_token))
}

async fn fetch_csrf_cookie(app: &axum::Router) -> TestResult<String> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/csrf")
                .body(Body::empty())?,
        )
        .await?;

    response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::to_owned)
        .ok_or_else(|| std::io::Error::other("set-cookie should contain a csrf cookie").into())
}

async fn login_response(
    app: &axum::Router,
    username: &str,
    password: &str,
) -> TestResult<axum::response::Response> {
    let csrf_cookie = fetch_csrf_cookie(app).await?;
    let csrf_token = csrf_cookie
        .strip_prefix("mini_conf_csrf=")
        .ok_or_else(|| std::io::Error::other("csrf cookie should have expected prefix"))?;

    Ok(app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::COOKIE, &csrf_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-csrf-token", csrf_token)
                .body(Body::from(format!(
                    r#"{{"username":{},"password":{}}}"#,
                    serde_json::to_string(username)?,
                    serde_json::to_string(password)?,
                )))?,
        )
        .await?)
}

#[tokio::test]
async fn login_requires_csrf_cookie_and_header() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let missing_csrf_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"admin","password":"admin123456"}"#,
                ))?,
        )
        .await?;
    assert_eq!(missing_csrf_response.status(), StatusCode::FORBIDDEN);
    let missing_payload: ErrorResponse = read_json(missing_csrf_response).await?;
    assert_eq!(missing_payload.code, "csrf_token_missing");

    let csrf_cookie = fetch_csrf_cookie(&app).await?;
    let invalid_csrf_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::COOKIE, &csrf_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-csrf-token", "wrong-token")
                .body(Body::from(
                    r#"{"username":"admin","password":"admin123456"}"#,
                ))?,
        )
        .await?;
    assert_eq!(invalid_csrf_response.status(), StatusCode::FORBIDDEN);
    let invalid_payload: ErrorResponse = read_json(invalid_csrf_response).await?;
    assert_eq!(invalid_payload.code, "csrf_token_invalid");

    teardown(&database_url, &schema, pool).await
}

fn cookie_value(response: &axum::response::Response, name: &str) -> TestResult<String> {
    for value in &response.headers().get_all(header::SET_COOKIE) {
        let raw = value.to_str()?;
        let Some(cookie) = raw.split(';').next() else {
            continue;
        };
        if let Some(cookie_value) = cookie.strip_prefix(&format!("{name}=")) {
            return Ok(format!("{name}={cookie_value}"));
        }
    }

    Err(std::io::Error::other(format!("missing cookie {name}")).into())
}

#[tokio::test]
async fn login_sets_session_cookie_and_returns_user_payload() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let response = login_response(&app, "admin", "admin123456").await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key(header::SET_COOKIE));
    assert!(cookie_value(&response, "mini_conf_csrf").is_ok());

    let payload: serde_json::Value = read_json(response).await?;
    assert_eq!(payload["user"]["username"], "admin");
    assert_eq!(payload["auth_mode"], "session");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn me_returns_current_user_from_session_cookie() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let login_response = login_response(&app, "admin", "admin123456").await?;
    let cookie = session_cookie(&login_response)?;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = read_json(response).await?;
    assert_eq!(payload["user"]["username"], "admin");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn logout_revokes_current_session() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let login_response = login_response(&app, "admin", "admin123456").await?;
    let (session, csrf_cookie, csrf_token) = auth_cookie_parts(&login_response)?;
    let cookie = format!("{session}; {csrf_cookie}");

    let logout_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/logout")
                .header(header::COOKIE, &cookie)
                .header("x-csrf-token", csrf_token)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(logout_response.status(), StatusCode::NO_CONTENT);

    let me_response = app
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header(header::COOKIE, session)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(me_response.status(), StatusCode::UNAUTHORIZED);
    let payload: ErrorResponse = read_json(me_response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "auth_session_expired".to_owned(),
            message: "Authentication session expired".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn change_password_clears_must_change_and_keeps_current_session() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    sqlx::query(
        r#"
        UPDATE users
        SET must_change_password = TRUE
        WHERE username = 'admin'
        "#,
    )
    .execute(&pool)
    .await?;

    let login_response = login_response(&app, "admin", "admin123456").await?;
    let (session, csrf_cookie, csrf_token) = auth_cookie_parts(&login_response)?;
    let cookie = format!("{session}; {csrf_cookie}");
    let login_payload: serde_json::Value = read_json(login_response).await?;
    assert_eq!(login_payload["user"]["must_change_password"], true);

    let change_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/change-password")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-csrf-token", &csrf_token)
                .body(Body::from(
                    r#"{"current_password":"admin123456","new_password":"NewPassword123"}"#,
                ))?,
        )
        .await?;

    assert_eq!(change_response.status(), StatusCode::OK);
    let change_payload: serde_json::Value = read_json(change_response).await?;
    assert_eq!(change_payload["user"]["must_change_password"], false);

    let me_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header(header::COOKIE, session)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(me_response.status(), StatusCode::OK);
    let me_payload: serde_json::Value = read_json(me_response).await?;
    assert_eq!(me_payload["user"]["must_change_password"], false);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn login_is_rate_limited_after_repeated_failures() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    for _ in 0..5 {
        let response = login_response(&app, "admin", "wrong-password").await?;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    let blocked_response = login_response(&app, "admin", "admin123456").await?;

    assert_eq!(blocked_response.status(), StatusCode::TOO_MANY_REQUESTS);
    let payload: ErrorResponse = read_json(blocked_response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "auth_rate_limited".to_owned(),
            message: "Too many failed login attempts; try again later".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn logout_requires_matching_csrf_token_when_cookie_is_present() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let login_response = login_response(&app, "admin", "admin123456").await?;
    let session = session_cookie(&login_response)?;
    let csrf = cookie_value(&login_response, "mini_conf_csrf")?;
    let cookie_header = format!("{session}; {csrf}");

    let missing_header_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/logout")
                .header(header::COOKIE, &cookie_header)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(missing_header_response.status(), StatusCode::FORBIDDEN);
    let missing_payload: ErrorResponse = read_json(missing_header_response).await?;
    assert_eq!(missing_payload.code, "csrf_token_missing");

    let invalid_header_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/logout")
                .header(header::COOKIE, &cookie_header)
                .header("x-csrf-token", "wrong-token")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(invalid_header_response.status(), StatusCode::FORBIDDEN);
    let invalid_payload: ErrorResponse = read_json(invalid_header_response).await?;
    assert_eq!(invalid_payload.code, "csrf_token_invalid");

    let csrf_value = csrf
        .strip_prefix("mini_conf_csrf=")
        .ok_or_else(|| std::io::Error::other("csrf cookie should have expected prefix"))?;
    let logout_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/logout")
                .header(header::COOKIE, &cookie_header)
                .header("x-csrf-token", csrf_value)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(logout_response.status(), StatusCode::NO_CONTENT);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn change_password_rejects_invalid_current_password() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let login_response = login_response(&app, "admin", "admin123456").await?;
    let (session, csrf_cookie, csrf_token) = auth_cookie_parts(&login_response)?;
    let cookie = format!("{session}; {csrf_cookie}");

    let change_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/change-password")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-csrf-token", csrf_token)
                .body(Body::from(
                    r#"{"current_password":"wrong-password","new_password":"NewPassword123"}"#,
                ))?,
        )
        .await?;

    assert_eq!(change_response.status(), StatusCode::UNAUTHORIZED);
    let payload: ErrorResponse = read_json(change_response).await?;
    assert_eq!(payload.code, "current_password_invalid");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn change_password_rejects_weak_new_password() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let login_response = login_response(&app, "admin", "admin123456").await?;
    let (session, csrf_cookie, csrf_token) = auth_cookie_parts(&login_response)?;
    let cookie = format!("{session}; {csrf_cookie}");

    let change_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/change-password")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-csrf-token", csrf_token)
                .body(Body::from(
                    r#"{"current_password":"admin123456","new_password":"aaaaaaaa"}"#,
                ))?,
        )
        .await?;

    assert_eq!(change_response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let payload: ErrorResponse = read_json(change_response).await?;
    assert_eq!(payload.code, "password_too_weak");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn must_change_password_blocks_app_workflows_until_changed() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    sqlx::query(
        r#"
        UPDATE users
        SET must_change_password = TRUE
        WHERE username = 'admin'
        "#,
    )
    .execute(&pool)
    .await?;

    let login_response = login_response(&app, "admin", "admin123456").await?;
    let (session, csrf_cookie, csrf_token) = auth_cookie_parts(&login_response)?;
    let cookie = format!("{session}; {csrf_cookie}");

    let projects_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects")
                .header(header::COOKIE, &session)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(projects_response.status(), StatusCode::CONFLICT);
    let payload: ErrorResponse = read_json(projects_response).await?;
    assert_eq!(payload.code, "password_change_required");

    let change_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/change-password")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-csrf-token", &csrf_token)
                .body(Body::from(
                    r#"{"current_password":"admin123456","new_password":"NewPassword123"}"#,
                ))?,
        )
        .await?;
    assert_eq!(change_response.status(), StatusCode::OK);

    let unlocked_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects")
                .header(header::COOKIE, session)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(unlocked_response.status(), StatusCode::OK);

    teardown(&database_url, &schema, pool).await
}
