//! Validated configuration, read once from the environment.
//!
//! Callers authenticate with Baobab workload identity only; there is no
//! unauthenticated mode and no static secret. The sandbox provider is
//! simulated and is refused in production (ADR-PAY-0001 section 15.2).

use std::collections::BTreeMap;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Integration,
    Staging,
    Production,
}

impl Environment {
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "development" => Some(Self::Development),
            "integration" => Some(Self::Integration),
            "staging" => Some(Self::Staging),
            "production" => Some(Self::Production),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Integration => "integration",
            Self::Staging => "staging",
            Self::Production => "production",
        }
    }
}

#[derive(Clone)]
pub struct Config {
    pub environment: Environment,
    pub http_port: u16,
    pub shutdown_grace: Duration,
    pub payment_provider: String,
    pub workload_issuer: String,
    pub workload_jwks_uri: String,
    pub workload_audience: String,
    /// Each allowed workload client and the engine it acts for: the engine
    /// whose obligations its payments must name as `source_engine`.
    pub allowed_clients: BTreeMap<String, String>,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("environment", &self.environment)
            .field("http_port", &self.http_port)
            .field("payment_provider", &self.payment_provider)
            .field("workload_issuer", &self.workload_issuer)
            .field("workload_audience", &self.workload_audience)
            .field("allowed_clients", &self.allowed_clients)
            .finish()
    }
}

pub const SANDBOX: &str = "sandbox";

impl Config {
    /// Reads configuration from `get` (the process environment in production).
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let mut problems = Vec::new();
        let environment = match get("BAOBAB_ENVIRONMENT").filter(|v| !v.trim().is_empty()) {
            None => {
                problems.push("BAOBAB_ENVIRONMENT is required".to_string());
                None
            }
            Some(raw) => Environment::parse(&raw).or_else(|| {
                problems.push(
                    "BAOBAB_ENVIRONMENT must be one of development, integration, staging, production".to_string(),
                );
                None
            }),
        };
        let http_port = get("HTTP_PORT")
            .unwrap_or_else(|| "8080".into())
            .trim()
            .parse::<u16>()
            .unwrap_or_else(|_| {
                problems.push("HTTP_PORT must be a port number".into());
                0
            });
        let grace = get("SHUTDOWN_GRACE_SECONDS")
            .unwrap_or_else(|| "10".into())
            .trim()
            .parse::<u64>()
            .ok()
            .filter(|s| *s <= 120)
            .unwrap_or_else(|| {
                problems.push("SHUTDOWN_GRACE_SECONDS must be between 0 and 120".into());
                0
            });

        let provider = get("PAYMENT_PROVIDER")
            .unwrap_or_else(|| SANDBOX.into())
            .trim()
            .to_ascii_lowercase();
        if provider != SANDBOX {
            problems.push(
                "PAYMENT_PROVIDER must be sandbox: the HyperSwitch adapter is not implemented yet"
                    .into(),
            );
        }
        if let Some(env) = environment {
            if env == Environment::Production {
                problems.push(
                    "the sandbox payment provider is simulated and is refused in production".into(),
                );
            }
            if matches!(env, Environment::Staging | Environment::Production) {
                problems.push(format!(
                    "{}: payment state is in-memory only; durable persistence is not implemented yet",
                    env.as_str()
                ));
            }
        }

        let issuer = required_url(&get, "WORKLOAD_ISSUER", &mut problems);
        let jwks = required_url(&get, "WORKLOAD_JWKS_URI", &mut problems);
        if let (Some(env), Some(uri)) = (environment, jwks.as_deref())
            && env != Environment::Development
            && !uri.starts_with("https://")
        {
            problems.push("WORKLOAD_JWKS_URI must use https outside development".into());
        }
        let audience = get("WORKLOAD_AUDIENCE")
            .unwrap_or_else(|| "baobab-payments".into())
            .trim()
            .to_string();
        if audience.is_empty() {
            problems.push("WORKLOAD_AUDIENCE must not be empty".into());
        }
        let mut allowed_clients = BTreeMap::new();
        for entry in get("WORKLOAD_ALLOWED_CLIENTS")
            .unwrap_or_else(|| DEFAULT_ALLOWED_CLIENTS.into())
            .split(',')
            .map(str::trim)
            .filter(|c| !c.is_empty())
        {
            match entry.split_once('=').map(|(c, e)| (c.trim(), e.trim())) {
                Some((client, engine))
                    if !client.is_empty()
                        && is_engine_key(engine)
                        && !allowed_clients.contains_key(client) =>
                {
                    allowed_clients.insert(client.to_string(), engine.to_string());
                }
                _ => problems.push(format!(
                    "WORKLOAD_ALLOWED_CLIENTS entry {entry:?} must be client_id=engine_key, once per client"
                )),
            }
        }
        if allowed_clients.is_empty() {
            problems.push("WORKLOAD_ALLOWED_CLIENTS must name at least one client".into());
        }
        if !problems.is_empty() {
            return Err(format!("invalid configuration: {}", problems.join("; ")));
        }
        Ok(Self {
            environment: environment.expect("checked above"),
            http_port,
            shutdown_grace: Duration::from_secs(grace),
            payment_provider: provider,
            workload_issuer: issuer.expect("checked above"),
            workload_jwks_uri: jwks.expect("checked above"),
            workload_audience: audience,
            allowed_clients,
        })
    }
}

/// The registered workload identity of baobab-subscriptions (Shared
/// `identity/v1/workload-registry.yaml`), acting for its own engine.
const DEFAULT_ALLOWED_CLIENTS: &str = "baobab-subscriptions-workload=baobab-subscriptions";

/// Shared `capability/v1` `engineKey`: `^[a-z][a-z0-9-]*$`, 2 to 63 characters.
fn is_engine_key(key: &str) -> bool {
    (2..=63).contains(&key.len())
        && key.starts_with(|c: char| c.is_ascii_lowercase())
        && key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn required_url(
    get: &impl Fn(&str) -> Option<String>,
    name: &str,
    problems: &mut Vec<String>,
) -> Option<String> {
    match get(name)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
    {
        None => {
            problems.push(format!("{name} is required"));
            None
        }
        Some(v) if v.starts_with("https://") || v.starts_with("http://") => Some(v),
        Some(_) => {
            problems.push(format!("{name} must be an absolute http(s) URL"));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k| map.get(k).cloned()
    }

    const IAM: [(&str, &str); 2] = [
        ("WORKLOAD_ISSUER", "https://iam.baobab.test/realms/baobab"),
        (
            "WORKLOAD_JWKS_URI",
            "https://iam.baobab.test/realms/baobab/certs",
        ),
    ];

    #[test]
    fn development_defaults() {
        let c = Config::from_env(env(&[
            ("BAOBAB_ENVIRONMENT", "development"),
            IAM[0],
            IAM[1],
        ]))
        .unwrap();
        assert_eq!(c.http_port, 8080);
        assert_eq!(c.workload_audience, "baobab-payments");
        assert_eq!(
            c.allowed_clients
                .get("baobab-subscriptions-workload")
                .map(String::as_str),
            Some("baobab-subscriptions")
        );
        assert_eq!(c.allowed_clients.len(), 1);
    }

    #[test]
    fn allowed_clients_name_their_engine() {
        let c = Config::from_env(env(&[
            ("BAOBAB_ENVIRONMENT", "development"),
            IAM[0],
            IAM[1],
            (
                "WORKLOAD_ALLOWED_CLIENTS",
                " baobab-subscriptions-workload = baobab-subscriptions , baobab-trade-workload=baobab-trade",
            ),
        ]))
        .unwrap();
        assert_eq!(c.allowed_clients["baobab-trade-workload"], "baobab-trade");
        assert_eq!(c.allowed_clients.len(), 2);
        for bad in [
            "baobab-subscriptions",
            "baobab-subscriptions-workload=",
            "=baobab-subscriptions",
            "baobab-subscriptions-workload=Baobab Subscriptions",
            "a=baobab-subscriptions,a=baobab-trade",
        ] {
            let e = Config::from_env(env(&[
                ("BAOBAB_ENVIRONMENT", "development"),
                IAM[0],
                IAM[1],
                ("WORKLOAD_ALLOWED_CLIENTS", bad),
            ]))
            .unwrap_err();
            assert!(e.contains("must be client_id=engine_key"), "{bad}: {e}");
        }
    }

    #[test]
    fn the_sandbox_is_refused_in_production() {
        let e = Config::from_env(env(&[("BAOBAB_ENVIRONMENT", "production"), IAM[0], IAM[1]]))
            .unwrap_err();
        assert!(e.contains("refused in production"), "{e}");
    }

    #[test]
    fn workload_identity_is_required() {
        let e = Config::from_env(env(&[("BAOBAB_ENVIRONMENT", "development")])).unwrap_err();
        assert!(
            e.contains("WORKLOAD_ISSUER is required")
                && e.contains("WORKLOAD_JWKS_URI is required"),
            "{e}"
        );
    }

    #[test]
    fn only_the_sandbox_exists() {
        let e = Config::from_env(env(&[
            ("BAOBAB_ENVIRONMENT", "development"),
            ("PAYMENT_PROVIDER", "hyperswitch"),
            IAM[0],
            IAM[1],
        ]))
        .unwrap_err();
        assert!(e.contains("not implemented"), "{e}");
    }

    #[test]
    fn integration_needs_https_keys() {
        let e = Config::from_env(env(&[
            ("BAOBAB_ENVIRONMENT", "integration"),
            IAM[0],
            ("WORKLOAD_JWKS_URI", "http://iam/certs"),
        ]))
        .unwrap_err();
        assert!(e.contains("https"), "{e}");
    }
}
