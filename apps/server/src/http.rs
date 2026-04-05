use axum::{Json, Router, extract::State, routing::get};
use infra::AppIdentity;
use schema::health::HealthzResponse;
use tower_http::trace::TraceLayer;

#[derive(Debug, Clone, Copy)]
struct AppState {
    identity: AppIdentity,
}

pub fn router(identity: AppIdentity) -> Router {
    Router::new()
        .route("/api/healthz", get(get_healthz))
        .with_state(AppState { identity })
        .layer(TraceLayer::new_for_http())
}

async fn get_healthz(State(state): State<AppState>) -> Json<HealthzResponse> {
    Json(HealthzResponse::ok(state.identity))
}
