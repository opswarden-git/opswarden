use opswarden_server::{build_app, config::Config};

#[tokio::main]
async fn main() {
    let config = Config::from_env();
    let app = build_app(config);

    let addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind address");
    println!("OpsWarden server listening on {addr}");
    axum::serve(listener, app).await.expect("server error");
}
