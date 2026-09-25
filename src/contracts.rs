//! Runtime validation against the baobab-platform/shared contracts this
//! engine pins. The files under `contracts/shared` are byte-for-byte copies of
//! the Shared commit in `contracts.lock.yaml`, embedded at build time and
//! registered under their canonical `$id`, so cross-file `$ref`s resolve
//! offline.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use jsonschema::{Draft, Registry, Validator};
use serde_json::{Value, json};

pub const BASE: &str = "https://contracts.baobab-platform.com/";
pub const PAYMENT: &str = "payments/v1/payment.schema.json";
pub const EVENTS: &str = "payments/v1/events.schema.json";
pub const ENVELOPE: &str = "events/v1/envelope.schema.json";
pub const PROBLEM: &str = "errors/v1/problem-details.schema.json";

macro_rules! embed {
    ($($path:literal),* $(,)?) => {
        &[$(($path, include_str!(concat!("../contracts/shared/", $path)))),*]
    };
}

/// Every vendored file, by its path under Shared's `contracts/`.
pub const EMBEDDED: &[(&str, &str)] = embed!(
    "capability/v1/domain.schema.json",
    "control-plane/v1/domain.schema.json",
    "errors/v1/problem-details.schema.json",
    "events/v1/envelope.schema.json",
    "organisation/v1/domain.schema.json",
    "payments/v1/capabilities.json",
    "payments/v1/domain.schema.json",
    "payments/v1/events.schema.json",
    "payments/v1/payment.schema.json",
);

/// A definition within a contract file, e.g. `def(PAYMENT, "PaymentIntent")`.
pub fn def(file: &str, definition: &str) -> String {
    format!("{file}#/$defs/{definition}")
}

fn registry() -> &'static Registry<'static> {
    static REGISTRY: OnceLock<Registry<'static>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
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
                    .with_draft(Draft::Draft202012)
                    .should_validate_formats(true)
                    .with_registry(registry())
                    .build(&schema)
                    .unwrap_or_else(|e| panic!("contract {reference} does not compile: {e}")),
            )
        })
        .clone()
}

/// The ways `instance` breaks the contract at `reference` (a path under
/// `contracts/`, optionally with a fragment). Empty when it conforms.
pub fn problems(reference: &str, instance: &Value) -> Vec<String> {
    let validator = validator(reference);
    let mut out: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| {
            let at = e.instance_path().to_string();
            format!("{}: {e}", if at.is_empty() { "$" } else { at.as_str() })
        })
        .collect();
    out.sort();
    out
}
