//! Current IAM/CP binding authority for the simulated sandbox runtime.
//! This gate conveys no legal mandate, PSP certification or permission to move
//! real money. The native backend retains its separate financial commit gates.
use crate::service::Refusal;
use reqwest::{Client, Url};
use serde_json::{Value, json};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

pub struct SandboxRequest<'a> {
    pub subject_token: &'a str,
    pub context_id: &'a str,
    pub tenant_id: &'a str,
    pub market_id: Option<&'a str>,
    pub capabilities: &'a [&'a str],
}
#[derive(Clone, Copy)]
pub struct SandboxLease {
    pub expires_at: OffsetDateTime,
}
impl SandboxLease {
    pub fn require_current(&self) -> Result<(), Refusal> {
        if self.expires_at <= OffsetDateTime::now_utc() {
            return Err(denied());
        }
        Ok(())
    }
}
pub trait SandboxAuthority: Send + Sync {
    fn authorise<'a>(
        &'a self,
        request: SandboxRequest<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<SandboxLease, Refusal>> + Send + 'a>>;
}
fn denied() -> Refusal {
    Refusal::new(
        403,
        "SANDBOX_CAPABILITY_DENIED",
        "current CP context and capability binding required",
    )
}
fn unavailable() -> Refusal {
    Refusal::new(
        503,
        "SANDBOX_AUTHORITY_UNAVAILABLE",
        "current CP authority unavailable",
    )
}
fn endpoint(raw: &str) -> Result<Url, Refusal> {
    let url = Url::parse(raw).map_err(|_| unavailable())?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(unavailable());
    }
    Ok(url)
}
fn timestamp(value: &Value, key: &str) -> Result<OffsetDateTime, Refusal> {
    OffsetDateTime::parse(value[key].as_str().ok_or_else(unavailable)?, &Rfc3339)
        .map_err(|_| unavailable())
}
fn canonical(value: &Value, key: &str, prefix: &str) -> bool {
    value[key].as_str().is_some_and(|v| {
        v.strip_prefix(prefix).is_some_and(|s| {
            s.len() == 32
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
    })
}
pub struct HttpSandboxAuthority {
    http: Client,
    token: Url,
    validate: Url,
    resolve: Url,
    validator_id: String,
    validator_secret: String,
    provider_id: String,
    instance_id: String,
}
impl HttpSandboxAuthority {
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Result<Option<Self>, Refusal> {
        let names = [
            "WORKLOAD_ISSUER",
            "SANDBOX_CP_ORIGIN",
            "SANDBOX_CP_CLIENT_ID",
            "SANDBOX_CP_CLIENT_SECRET",
            "SANDBOX_CAPABILITY_PROVIDER_ID",
            "SANDBOX_ENGINE_INSTANCE_ID",
        ];
        let values: Vec<Option<String>> = names.iter().map(|n| get(n)).collect();
        if values[1..].iter().all(Option::is_none) {
            return Ok(None);
        }
        let values: Vec<String> = values
            .into_iter()
            .map(|v| {
                v.filter(|s| !s.is_empty() && s.len() <= 8192 && !s.contains(['\r', '\n']))
                    .ok_or_else(unavailable)
            })
            .collect::<Result<_, _>>()?;
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none());
        if let Some(path) = get("SSL_CERT_FILE") {
            let pem = std::fs::read(path).map_err(|_| unavailable())?;
            if pem.len() > 65536 {
                return Err(unavailable());
            }
            for cert in reqwest::Certificate::from_pem_bundle(&pem).map_err(|_| unavailable())? {
                builder = builder.add_root_certificate(cert)
            }
        }
        let ids = json!({"provider":values[4],"instance":values[5]});
        if !canonical(&ids, "provider", "provider_") || !canonical(&ids, "instance", "ei_") {
            return Err(unavailable());
        }
        Ok(Some(Self {
            http: builder.build().map_err(|_| unavailable())?,
            token: endpoint(&(values[0].trim_end_matches('/').to_owned() + "/oauth2/token"))?,
            validate: endpoint(
                &(values[1].trim_end_matches('/').to_owned() + "/v1/platform-context/validate"),
            )?,
            resolve: endpoint(
                &(values[1].trim_end_matches('/').to_owned() + "/v1/capabilities/resolve"),
            )?,
            validator_id: values[2].clone(),
            validator_secret: values[3].clone(),
            provider_id: values[4].clone(),
            instance_id: values[5].clone(),
        }))
    }
    async fn object(&self, mut response: reqwest::Response) -> Result<Value, Refusal> {
        let status = response.status().as_u16();
        if status == 401 {
            return Err(Refusal::new(
                401,
                "AUTH_TOKEN_INVALID",
                "current subject identity refused",
            ));
        }
        if matches!(status, 403 | 404) {
            return Err(denied());
        }
        if status != 200 || response.content_length().is_some_and(|s| s > 16384) {
            return Err(unavailable());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| unavailable())? {
            if chunk.len() > 16384 - bytes.len() {
                return Err(unavailable());
            }
            bytes.extend_from_slice(&chunk);
        }
        crate::wire_json::decode_object(&bytes).map_err(|_| unavailable())
    }
    async fn current(&self, r: SandboxRequest<'_>) -> Result<SandboxLease, Refusal> {
        if Uuid::parse_str(r.context_id).is_err()
            || r.subject_token.is_empty()
            || r.tenant_id.is_empty()
            || r.capabilities.is_empty()
        {
            return Err(denied());
        }
        // No positive authority cache. The registered validator redeems the
        // actual native request's bearer, including current issuer revocation.
        let response=self.http.post(self.token.clone()).header("Content-Type","application/x-www-form-urlencoded")
            .body(format!("grant_type=client_credentials&audience=baobab-control-plane&scope=context%3Avalidate&client_id={}&client_secret={}",form(&self.validator_id),form(&self.validator_secret)))
            .send().await.map_err(|_|unavailable())?;
        let issued = self.object(response).await?;
        let validator = issued["access_token"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 8192)
            .ok_or_else(unavailable)?;
        let response = self
            .http
            .post(self.validate.clone())
            .bearer_auth(validator)
            .json(&json!({"context_id":r.context_id,"subject_token":r.subject_token}))
            .send()
            .await
            .map_err(|_| unavailable())?;
        let context = self.object(response).await?;
        let now = OffsetDateTime::now_utc();
        let mut until = timestamp(&context, "expires_at")?;
        let context_expiry = until;
        if context["context_id"] != r.context_id
            || context["tenant_id"] != r.tenant_id
            || context["authority_purpose"] != "RUNTIME"
            || r.market_id
                .is_some_and(|market| context["market_id"] != market)
            || until <= now
            || until > now + time::Duration::minutes(15)
        {
            return Err(denied());
        }
        for capability in r.capabilities {
            let correlation = Uuid::now_v7().to_string();
            let response=self.http.post(self.resolve.clone()).bearer_auth(r.subject_token).json(&json!({"context_id":r.context_id,"capability_key":capability,"required_contract_version":1,"correlation_id":correlation})).send().await.map_err(|_|unavailable())?;
            let decision = self.object(response).await?;
            if decision["decision"] != "RESOLVED" {
                return Err(denied());
            }
            let observed = timestamp(&decision, "resolved_at")?;
            let expiry = timestamp(&decision, "expires_at")?;
            let native = &decision["invocation"];
            if decision["context_id"] != r.context_id
                || decision["capability_key"] != *capability
                || decision["correlation_id"] != correlation
                || decision["contract_version"].as_u64() != Some(1)
                || native["contract_version"].as_u64() != Some(1)
                || native["provider_id"] != self.provider_id
                || native["engine_instance_id"] != self.instance_id
                || native["service_reference"] != "service://baobab-payments/payments"
                || native["protocol"] != "http"
                || !canonical(&decision, "resolution_id", "res_")
                || !canonical(&decision, "grant_id", "grant_")
                || !canonical(&decision, "binding_id", "bind_")
                || observed < now - time::Duration::seconds(5)
                || observed > OffsetDateTime::now_utc() + time::Duration::seconds(1)
                || expiry <= observed
                || expiry > context_expiry
            {
                return Err(unavailable());
            }
            until = until.min(expiry);
        }
        let lease = SandboxLease { expires_at: until };
        lease.require_current()?;
        Ok(lease)
    }
}
fn form(raw: &str) -> String {
    let mut out = String::new();
    for b in raw.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(char::from(b))
        } else {
            use std::fmt::Write;
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}
impl SandboxAuthority for HttpSandboxAuthority {
    fn authorise<'a>(
        &'a self,
        r: SandboxRequest<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<SandboxLease, Refusal>> + Send + 'a>> {
        Box::pin(self.current(r))
    }
}
