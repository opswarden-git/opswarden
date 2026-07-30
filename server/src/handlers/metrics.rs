use axum::{
    extract::State,
    http::{header, HeaderValue},
    response::IntoResponse,
};

use crate::AppState;

pub async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8"),
        )],
        state.alertmanager_metrics.render_prometheus(),
    )
}
