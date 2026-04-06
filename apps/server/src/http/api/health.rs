use crate::state::AppState;
use axum::{Json, Router, extract::State, routing::get};
use schema::health::HealthzResponse;

pub fn router() -> Router<AppState> {
    Router::new().route("/healthz", get(get_healthz))
}

#[utoipa::path(
    get,
    path = "/api/healthz",
    tag = "system",
    responses(
        (status = 200, description = "Service health check", body = HealthzResponse)
    )
)]
pub(crate) async fn get_healthz(State(state): State<AppState>) -> Json<HealthzResponse> {
    Json(HealthzResponse::ok(state.identity()))
}
