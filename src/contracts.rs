//! Runtime validation against the baobab-platform/shared contracts this
//! engine pins. The files under `contracts/shared` are byte-for-byte copies of
//! the Shared commit in `contracts.lock.yaml`, embedded at build time and
//! registered under their canonical `$id`, so cross-file `$ref`s resolve
//! offline.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use jsonschema::{Draft, Registry, Validator};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub const BASE: &str = "https://contracts.baobab-platform.com/";
pub const PAYMENT: &str = "payments/v1/payment.schema.json";
pub const EVENTS: &str = "payments/v1/events.schema.json";
pub const ENVELOPE: &str = "events/v1/envelope.schema.json";
pub const PROBLEM: &str = "errors/v1/problem-details.schema.json";

// Generated include inventory prevents a lock bump from silently leaving an
// older or incomplete runtime bundle. These are compiled into the executable.
include!("contracts_embedded.rs");

/// A definition within a contract file, e.g. `def(PAYMENT, "PaymentIntent")`.
pub fn def(file: &str, definition: &str) -> String {
    format!("{file}#/$defs/{definition}")
}

fn registry() -> &'static Registry<'static> {
    static REGISTRY: OnceLock<Registry<'static>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        verify_provenance();
        let mut builder = Registry::new();
        for (path, text) in EMBEDDED.iter().filter(|(p, _)| p.ends_with(".schema.json")) {
            let value: Value = serde_json::from_str(text).expect("vendored contract is JSON");
            builder = builder
                .add(format!("{BASE}{path}"), value)
                .expect("vendored contract URI is valid");
        }
        builder
            .prepare()
            .expect("vendored contracts form a registry")
    })
}

fn validator(reference: &str) -> Arc<Validator> {
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<Validator>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let mut cache = cache.lock().expect("validator cache poisoned");
    cache
        .entry(reference.to_string())
        .or_insert_with(|| {
            let schema = json!({ "$ref": format!("{BASE}{reference}") });
            Arc::new(
                jsonschema::options()
                    .offline()
                    .with_draft(Draft::Draft202012)
                    .should_validate_formats(true)
                    .with_registry(registry())
                    .build(&schema)
                    .unwrap_or_else(|e| panic!("contract {reference} does not compile: {e}")),
            )
        })
        .clone()
}

/// Schema locations only: errors must not copy tokens, payment references,
/// evidence values or attacker-selected property names into responses/logs.
pub fn problems(reference: &str, instance: &Value) -> Vec<String> {
    let validator = validator(reference);
    let mut out: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| format!("contract violation at {}", e.schema_path()))
        .collect();
    out.sort();
    out
}

/// The declared owner entry is checked before independent operation/actor,
/// freshness and provider rules. Wire validity never supplies authority.
pub fn entry_is_valid(name: &str, instance: &Value) -> bool {
    let reference = ENTRIES
        .iter()
        .find(|(entry, _)| *entry == name)
        .expect("declared contract entry")
        .1;
    validator(reference).is_valid(instance)
}

fn verify_provenance() {
    let manifest: Value = serde_json::from_str(PROVENANCE).expect("generated provenance JSON");
    assert_eq!(manifest["commit"].as_str(), Some(SHARED_COMMIT));
    let files = manifest["files"].as_object().expect("generated inventory");
    assert_eq!(files.len(), EMBEDDED.len(), "complete embedded inventory");
    for (path, text) in EMBEDDED {
        let digest = hex::encode(Sha256::digest(text.as_bytes()));
        assert_eq!(files[*path].as_str(), Some(digest.as_str()), "schema hash");
    }
}

/// Build every declared validator and assert UUID/date-time behaviour in the
/// standalone binary, without issuer credentials, Shared checkout or services.
pub fn self_test() {
    verify_provenance();
    for (_, reference) in ENTRIES {
        let _ = validator(reference);
    }
    assert!(entry_is_valid("context_uuid", &json!("0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6b")));
    assert!(!entry_is_valid("context_uuid", &json!("not-a-uuid")));
    assert!(!entry_is_valid("context_response", &json!({
        "context_id": "0199a1b2-c3d4-7e8f-9a0b-1c2d3e4f5a6b",
        "tenant_id": "tn_synthetic123", "resolved_at": "2026-02-30T00:00:00Z"
    })));
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireCase {
    name: String,
    contract: String,
    instance: Value,
    valid: bool,
}

/// Synthetic golden vectors only; reports a fixed error rather than instances.
pub fn check_conformance(cases: &str) -> Result<usize, &'static str> {
    let cases: Vec<WireCase> = serde_json::from_str(cases).map_err(|_| "invalid conformance vectors")?;
    if cases.is_empty() {
        return Err("empty conformance vectors");
    }
    for case in &cases {
        if case.name.is_empty() || !ENTRIES.iter().any(|(name, _)| *name == case.contract) {
            return Err("unknown conformance entry");
        }
        if entry_is_valid(&case.contract, &case.instance) != case.valid {
            return Err("conformance result differs from pinned owner");
        }
    }
    Ok(cases.len())
}
