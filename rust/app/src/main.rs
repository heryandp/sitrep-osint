mod middleware;
mod original;
mod units;

use std::net::SocketAddr;

use axum::serve::ListenerExt;
use original::shutdown_signal;

fn load_dotenv() {
    if let Ok(content) = std::fs::read_to_string(".env") {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let key = k.trim();
                let mut val = v.trim();
                if (val.starts_with('"') && val.ends_with('"'))
                    || (val.starts_with('\'') && val.ends_with('\''))
                {
                    val = &val[1..val.len() - 1];
                }
                if std::env::var(key).is_err() {
                    // SAFETY: Single-threaded initialization before any thread or Tokio runtime starts.
                    unsafe {
                        std::env::set_var(key, val);
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    load_dotenv();
    original::report_fallbacks();
    let original = original::Original::start().await;
    let app = units::router()
        .layer(axum::middleware::from_fn(middleware::rate_limit_middleware))
        .with_state(original.clone());
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .expect("binding the port");
    let listener = listener.tap_io(|tcp| {
        let _ = tcp.set_nodelay(true);
    });
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("serving");
    original.stop();
}
