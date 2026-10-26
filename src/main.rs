use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use baobab_payments::auth::{Authenticator, Keys, RemoteJwks};
use baobab_payments::config::{Config, NativeBackendConfig};
use baobab_payments::cp_authority::IamCpAuthority;
use baobab_payments::http::{AppState, router};
use baobab_payments::hyperswitch::HyperSwitchTransport;
use baobab_payments::merchant_binding::MerchantBindings;
use baobab_payments::money_movement_execution::MoneyMovementExecutor;
use baobab_payments::native_api::NativePaymentApi;
use baobab_payments::native_execution::NativeTransactionService;
use baobab_payments::provider::SandboxProvider;
use baobab_payments::service::PaymentService;

/// A secret from a mounted file, without its trailing newline.
fn secret(path: &str, name: &str) -> Result<String, String> {
    let value = std::fs::read_to_string(path).map_err(|_| format!("{name} cannot be read"))?;
    let value = value.trim_end_matches(['\r', '\n']).to_string();
    if value.is_empty() {
        return Err(format!("{name} is empty"));
    }
    Ok(value)
}

/// P0-04B Gate D1: the native backend. Each constructor refuses anything but
/// a local integration composition: loopback PostgreSQL, TLS to HyperSwitch,
/// CP and IAM, and an environment that configuration already limited to
/// development or integration.
fn native_backend(
    config: &NativeBackendConfig,
    environment: &str,
) -> Result<NativePaymentApi, String> {
    let database_url = secret(&config.database_url_file, "PAYMENTS_DATABASE_URL_FILE")?;
    let api_key = secret(&config.hyperswitch_api_key_file, "HYPERSWITCH_API_KEY_FILE")?;
    let workload_secret = secret(
        &config.workload_secret_file,
        "PAYMENTS_WORKLOAD_SECRET_FILE",
    )?;
    let bindings = std::fs::read_to_string(&config.merchant_bindings_file)
        .map_err(|_| "MERCHANT_BINDINGS_FILE cannot be read".to_string())
        .and_then(|raw| MerchantBindings::parse(&raw))?;
    let transport = || {
        HyperSwitchTransport::new(&config.hyperswitch_origin, api_key.clone())
            .map_err(|_| "HYPERSWITCH_ORIGIN or its API key is not acceptable".to_string())
    };
    let executor = MoneyMovementExecutor::local_integration(
        database_url.clone(),
        transport()?,
        environment,
        config.engine_instance_id.clone(),
    )
    .map_err(|_| "the money-movement executor refuses this composition".to_string())?;
    let intents = NativeTransactionService::local_integration(
        database_url.clone(),
        transport()?,
        environment,
    )
    .map_err(|_| "the intent journal refuses this composition".to_string())?;
    let cp = IamCpAuthority::new(
        &config.cp_origin,
        &config.iam_token_endpoint,
        config.workload_client_id.clone(),
        workload_secret,
    )
    .map_err(|_| {
        "CP_ORIGIN, IAM_TOKEN_ENDPOINT or the workload credentials are not acceptable".to_string()
    })?;
    Ok(NativePaymentApi::new(
        database_url,
        executor,
        intents,
        cp,
        bindings,
    ))
}

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
    // Offline packaging/CI probes run without constructing a payment runtime.
    match std::env::args().nth(1).as_deref() {
        Some("contract-self-test") => {
            baobab_payments::contracts::self_test();
            return ExitCode::SUCCESS;
        }
        Some("contract-conformance") => {
            let result = std::env::args()
                .nth(2)
                .and_then(|path| std::fs::read_to_string(path).ok())
                .and_then(|cases| baobab_payments::contracts::check_conformance(&cases).ok());
            return if let Some(count) = result {
                println!("Pinned Shared conformance: {count} synthetic cases");
                ExitCode::SUCCESS
            } else {
                eprintln!("Pinned Shared conformance failed");
                ExitCode::FAILURE
            };
        }
        _ => {}
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
    let jwks = match RemoteJwks::new(config.workload_jwks_uri.clone()) {
        Ok(keys) => keys,
        Err(reason) => {
            tracing::error!(event = "startup.refused", reason);
            return ExitCode::from(2);
        }
    };
    let auth = Arc::new(Authenticator::new(
        Keys::Remote(jwks),
        config.workload_issuer.clone(),
        config.workload_audience.clone(),
        config.allowed_clients.clone(),
    ));
    let native = match config.native.as_ref() {
        None => None,
        Some(native) => match native_backend(native, config.environment.as_str()) {
            Ok(api) => Some(Arc::new(api)),
            Err(reason) => {
                tracing::error!(event = "startup.refused", reason);
                return ExitCode::from(2);
            }
        },
    };
    let sandbox_authority = if native.is_none() {
        match baobab_payments::sandbox_authority::HttpSandboxAuthority::from_env(|k| {
            env.get(k).cloned()
        }) {
            Ok(value) => value.map(|v| {
                Arc::new(v) as Arc<dyn baobab_payments::sandbox_authority::SandboxAuthority>
            }),
            Err(_) => {
                tracing::error!(
                    event = "startup.refused",
                    reason = "invalid sandbox authority configuration"
                );
                return ExitCode::from(2);
            }
        }
    } else {
        None
    };
    let native_backend_configured = native.is_some();
    let app = router(AppState {
        service,
        auth,
        environment: config.environment.as_str(),
        sandbox_authority,
        native,
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
        payment_provider = if native_backend_configured {
            "HYPERSWITCH"
        } else {
            "SANDBOX"
        },
        simulated = !native_backend_configured,
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
