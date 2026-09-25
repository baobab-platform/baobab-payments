use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use baobab_payments::auth::{Authenticator, Keys, RemoteJwks};
use baobab_payments::config::Config;
use baobab_payments::http::{AppState, router};
use baobab_payments::provider::SandboxProvider;
use baobab_payments::service::PaymentService;

/// `baobab-payments healthcheck`: the container HEALTHCHECK, needing no shell
/// or curl in the image. Exits 0 when /health/live answers 200.
fn healthcheck() -> ExitCode {
    let port = std::env::var("HTTP_PORT").unwrap_or_else(|_| "8080".into());
    let probe = || -> std::io::Result<bool> {
        let mut stream = TcpStream::connect_timeout(
            &format!("127.0.0.1:{port}")
                .parse()
                .map_err(std::io::Error::other)?,
            Duration::from_secs(2),
        )?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.write_all(
            b"GET /health/live HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        )?;
        let mut head = [0u8; 12];
        stream.read_exact(&mut head)?;
        Ok(head.starts_with(b"HTTP/1.1 200"))
    };
    if probe().unwrap_or(false) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        return healthcheck();
    }
    tracing_subscriber::fmt()
        .json()
        .flatten_event(true)
        .with_current_span(false)
        .with_target(false)
        .init();

    let env: HashMap<String, String> = std::env::vars().collect();
    let config = match Config::from_env(|k| env.get(k).cloned()) {
        Ok(config) => config,
        Err(reason) => {
            tracing::error!(event = "startup.refused", reason);
            return ExitCode::from(2);
        }
    };
    let provider = Arc::new(SandboxProvider);
    let service = Arc::new(PaymentService::new(
        provider.clone(),
        Arc::new(time::OffsetDateTime::now_utc),
    ));
    let auth = Arc::new(Authenticator::new(
        Keys::Remote(RemoteJwks::new(config.workload_jwks_uri.clone())),
        config.workload_issuer.clone(),
        config.workload_audience.clone(),
        config.allowed_clients.clone(),
    ));
    let app = router(AppState {
        service,
        auth,
        environment: config.environment.as_str(),
    });
    let listener = match tokio::net::TcpListener::bind(("0.0.0.0", config.http_port)).await {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!(event = "startup.failed", error = %e);
            return ExitCode::FAILURE;
        }
    };
    tracing::info!(
        event = "startup.completed",
        port = config.http_port,
        environment = config.environment.as_str(),
        payment_provider = "SANDBOX",
        simulated = true,
    );
    let grace = config.shutdown_grace;
    let (stopping, mut stopped) = tokio::sync::watch::channel(false);
    let served = axum::serve(listener, app).with_graceful_shutdown(async move {
        shutdown_signal().await;
        tracing::info!(event = "shutdown.started", grace_seconds = grace.as_secs());
        let _ = stopping.send(true);
    });
    // In-flight requests finish; one still running after the grace period is abandoned.
    let deadline = async move {
        let _ = stopped.changed().await;
        tokio::time::sleep(grace).await;
    };
    tokio::select! {
        result = served => match result {
            Ok(()) => {
                tracing::info!(event = "shutdown.completed");
                ExitCode::SUCCESS
            }
            Err(e) => {
                tracing::error!(event = "server.failed", error = %e);
                ExitCode::FAILURE
            }
        },
        () = deadline => {
            tracing::warn!(event = "shutdown.grace_expired");
            ExitCode::FAILURE
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut signal) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            signal.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
