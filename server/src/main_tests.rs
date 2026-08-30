use super::*;
use axum::{body::Body, routing::get, Router};
use std::{io, sync::Mutex};
use tower::ServiceExt;
use tracing_subscriber::fmt::format::FmtSpan;

struct TraceWriter(Arc<Mutex<Vec<u8>>>);

impl io::Write for TraceWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn completed_component_finishes_before_the_deadline() {
    let mut task = tokio::spawn(async { 42 });
    let result = wait_for_component(
        "test component",
        &mut task,
        Instant::now() + Duration::from_secs(1),
    )
    .await;

    assert_eq!(result.unwrap().unwrap(), 42);
}

#[tokio::test]
async fn stalled_component_is_aborted_at_the_deadline() {
    let mut task = tokio::spawn(std::future::pending::<()>());
    let result = wait_for_component(
        "test component",
        &mut task,
        Instant::now() + Duration::from_millis(10),
    )
    .await;

    assert!(result.is_none());
    assert!(task.is_finished());
}

#[tokio::test]
async fn http_traces_exclude_query_and_request_secrets() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let writer = output.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || TraceWriter(writer.clone()))
        .with_ansi(false)
        .without_time()
        .with_span_events(FmtSpan::FULL)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let app = Router::new().route("/callback", get(|| async {})).layer(
        TraceLayer::new_for_http()
            .make_span_with(|request: &axum::http::Request<_>| http_request_span(request)),
    );
    let request = axum::http::Request::builder()
        .uri("/callback?code=authorization-code-secret&state=oauth-state-secret")
        .header("cookie", "oauth=secret-cookie")
        .header("authorization", "Bearer secret-header")
        .body(Body::empty())
        .unwrap();

    app.oneshot(request).await.unwrap();
    let traces = String::from_utf8(output.lock().unwrap().clone()).unwrap();
    assert!(traces.contains("/callback"));
    for secret in [
        "authorization-code-secret",
        "oauth-state-secret",
        "secret-cookie",
        "secret-header",
    ] {
        assert!(!traces.contains(secret), "trace leaked {secret}");
    }
}
