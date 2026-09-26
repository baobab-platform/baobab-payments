//! Baobab workload identity (ADR-PAY-0001): a signed JWT from the configured
//! issuer, for this engine's audience, with `actor_type: workload`, from an
//! allowed client, holding the route's scope and living at most 15 minutes.
//! Static bearer secrets are not accepted.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::{Duration, Instant};

use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::Deserialize;
use tokio::sync::RwLock;

const MAX_LIFETIME_SECONDS: i64 = 900;
const ALGORITHMS: [Algorithm; 3] = [Algorithm::RS256, Algorithm::PS256, Algorithm::ES256];

/// The authenticated workload.
#[derive(Debug, Clone)]
pub struct Caller {
    pub subject: String,
    pub client_id: String,
    /// The engine the client acts for, from configuration, never the token.
    pub engine: String,
    pub scopes: BTreeSet<String>,
}

/// Why a caller was refused: 401 (not authenticated) or 403 (not authorised).
#[derive(Debug, Clone, PartialEq)]
pub struct AuthError {
    pub status: u16,
    pub code: &'static str,
    pub detail: &'static str,
}

fn invalid() -> AuthError {
    AuthError {
        status: 401,
        code: "AUTH_TOKEN_INVALID",
        detail: "the bearer token is invalid",
    }
}

#[derive(Deserialize)]
struct Claims {
    sub: String,
    iat: i64,
    exp: i64,
    jti: Option<String>,
    scope: Option<String>,
    actor_type: Option<String>,
    azp: Option<String>,
    client_id: Option<String>,
}

/// Where verification keys come from.
pub enum Keys {
    /// Fixed keys by `kid` (tests and local tooling).
    Static(HashMap<String, DecodingKey>),
    /// The issuer's JWKS endpoint, cached and refreshed on an unknown `kid`.
    Remote(RemoteJwks),
}

pub struct RemoteJwks {
    uri: String,
    client: reqwest::Client,
    cache: RwLock<(Option<Instant>, HashMap<String, DecodingKey>)>,
}

const JWKS_TTL: Duration = Duration::from_secs(300);
const JWKS_MIN_REFRESH: Duration = Duration::from_secs(30);

impl RemoteJwks {
    pub fn new(uri: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("an HTTP client builds");
        Self {
            uri,
            client,
            cache: RwLock::new((None, HashMap::new())),
        }
    }

    async fn key(&self, kid: &str) -> Option<DecodingKey> {
        {
            let cache = self.cache.read().await;
            if let (Some(fetched), Some(key)) = (cache.0, cache.1.get(kid))
                && fetched.elapsed() < JWKS_TTL
            {
                return Some(key.clone());
            }
            if cache
                .0
                .is_some_and(|fetched| fetched.elapsed() < JWKS_MIN_REFRESH)
            {
                return cache.1.get(kid).cloned();
            }
        }
        let mut cache = self.cache.write().await;
        if let Ok(response) = self.client.get(&self.uri).send().await
            && let Ok(response) = response.error_for_status()
            && let Ok(set) = response.json::<JwkSet>().await
        {
            let keys = set
                .keys
                .iter()
                .filter_map(|jwk| {
                    Some((jwk.common.key_id.clone()?, DecodingKey::from_jwk(jwk).ok()?))
                })
                .collect();
            *cache = (Some(Instant::now()), keys);
        }
        cache.1.get(kid).cloned()
    }
}

pub struct Authenticator {
    keys: Keys,
    issuer: String,
    audience: String,
    allowed_clients: BTreeMap<String, String>,
}

impl Authenticator {
    pub fn new(
        keys: Keys,
        issuer: String,
        audience: String,
        allowed_clients: BTreeMap<String, String>,
    ) -> Self {
        Self {
            keys,
            issuer,
            audience,
            allowed_clients,
        }
    }

    /// The caller behind an Authorization header, holding `scope`.
    pub async fn authenticate(
        &self,
        authorization: Option<&str>,
        scope: &str,
    ) -> Result<Caller, AuthError> {
        let token = authorization
            .and_then(|h| {
                h.strip_prefix("Bearer ")
                    .or_else(|| h.strip_prefix("bearer "))
            })
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .ok_or(AuthError {
                status: 401,
                code: "AUTH_TOKEN_REQUIRED",
                detail: "a bearer token is required",
            })?;
        let header = decode_header(token).map_err(|_| invalid())?;
        if !ALGORITHMS.contains(&header.alg) {
            return Err(invalid());
        }
        let kid = header.kid.ok_or_else(invalid)?;
        let key = match &self.keys {
            Keys::Static(keys) => keys.get(&kid).cloned(),
            Keys::Remote(remote) => remote.key(&kid).await,
        }
        .ok_or_else(invalid)?;
        let mut validation = Validation::new(header.alg);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);
        validation.set_required_spec_claims(&["exp", "iat", "sub", "iss", "aud"]);
        validation.leeway = 30;
        let claims = decode::<Claims>(token, &key, &validation)
            .map_err(|_| invalid())?
            .claims;
        if claims.exp - claims.iat > MAX_LIFETIME_SECONDS
            || claims.jti.as_deref().is_none_or(str::is_empty)
        {
            return Err(invalid());
        }
        let scopes: BTreeSet<String> = claims
            .scope
            .as_deref()
            .unwrap_or_default()
            .split(' ')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        let client = claims.azp.or(claims.client_id);
        let engine = client.as_ref().and_then(|c| self.allowed_clients.get(c));
        match (client, engine) {
            (Some(client), Some(engine))
                if claims.actor_type.as_deref() == Some("workload") && scopes.contains(scope) =>
            {
                Ok(Caller {
                    subject: claims.sub,
                    engine: engine.clone(),
                    client_id: client,
                    scopes,
                })
            }
            _ => Err(AuthError {
                status: 403,
                code: "AUTHORIZATION_DENIED",
                detail: "the authenticated principal lacks required authority",
            }),
        }
    }
}
