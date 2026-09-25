//! Validated configuration, read once from the environment.
//!
//! Callers authenticate with Baobab workload identity only; there is no
//! unauthenticated mode and no static secret. The sandbox provider is
//! simulated and is refused in production (ADR-PAY-0001 section 15.2).

use std::collections::BTreeSet;
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
    pub allowed_clients: BTreeSet<String>,
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
        let allowed_clients: BTreeSet<String> = get("WORKLOAD_ALLOWED_CLIENTS")
            .unwrap_or_else(|| "baobab-subscriptions".into())
            .split(',')
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
            .collect();
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
        assert!(c.allowed_clients.contains("baobab-subscriptions"));
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
