//! P0-04B Gate D1: the merchant each tenant collects through on the native
//! backend. Bindings are deployment configuration reviewed with the provider
//! activation, never request input: a caller can only name a tenant, and the
//! binding decides the responsible legal entity, operating organisation,
//! market, currency and provider profile its money moves under.

use std::collections::BTreeMap;

use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantBinding {
    pub tenant_id: String,
    pub responsible_legal_entity_id: String,
    pub operating_organisation_id: Uuid,
    pub market: String,
    pub currency: String,
    /// The provider profile (HyperSwitch `profile_id`) money moves under.
    pub merchant_profile_reference: String,
    /// The provider merchant account the profile belongs to.
    pub provider_merchant_id: String,
    pub provider_engine_instance_id: String,
    pub certification_reference: String,
    pub merchant_activation_reference: String,
    /// The platform accounts that may pay this merchant; each is also the
    /// provider customer that holds its saved payment methods.
    pub platform_account_ids: Vec<String>,
}

fn printable(value: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&value.len()) && value.bytes().all(|b| (0x21..=0x7e).contains(&b))
}

fn lower_alnum_id(value: &str, prefix: &str, min: usize) -> bool {
    value.len() <= 63
        && value.len() >= min
        && value.strip_prefix(prefix).is_some_and(|rest| {
            rest.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

impl MerchantBinding {
    fn is_valid(&self) -> bool {
        lower_alnum_id(&self.tenant_id, "tn_", 6)
            && printable(&self.responsible_legal_entity_id, 3, 63)
            && self.market.len() == 2
            && self.market.bytes().all(|b| b.is_ascii_uppercase())
            && self.currency.len() == 3
            && self.currency.bytes().all(|b| b.is_ascii_uppercase())
            && printable(&self.merchant_profile_reference, 3, 160)
            && printable(&self.provider_merchant_id, 3, 160)
            && printable(&self.provider_engine_instance_id, 3, 160)
            && printable(&self.certification_reference, 3, 256)
            && printable(&self.merchant_activation_reference, 3, 256)
            && !self.platform_account_ids.is_empty()
            && self
                .platform_account_ids
                .iter()
                .all(|p| lower_alnum_id(p, "pacct_", 7))
    }

    pub fn allows_platform_account(&self, platform_account_id: &str) -> bool {
        self.platform_account_ids
            .iter()
            .any(|p| p == platform_account_id)
    }
}

/// One binding per tenant.
#[derive(Debug, Clone, Default)]
pub struct MerchantBindings(BTreeMap<String, MerchantBinding>);

impl MerchantBindings {
    /// Parses `{"bindings": [...]}`. Any invalid or duplicated binding
    /// refuses the whole file: a partial merchant map is never served.
    pub fn parse(raw: &str) -> Result<Self, String> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct File {
            bindings: Vec<MerchantBinding>,
        }
        let file: File = serde_json::from_str(raw)
            .map_err(|_| "merchant bindings are not valid JSON".to_string())?;
        let mut map = BTreeMap::new();
        for binding in file.bindings {
            if !binding.is_valid() {
                return Err(format!(
                    "merchant binding for {:?} is invalid",
                    binding.tenant_id
                ));
            }
            if map.insert(binding.tenant_id.clone(), binding).is_some() {
                return Err("a tenant has more than one merchant binding".into());
            }
        }
        if map.is_empty() {
            return Err("merchant bindings name no tenant".into());
        }
        Ok(Self(map))
    }

    pub fn get(&self, tenant_id: &str) -> Option<&MerchantBinding> {
        self.0.get(tenant_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(extra: &str) -> String {
        format!(
            r#"{{"bindings":[{{"tenant_id":"tn_d1za","responsible_legal_entity_id":"le_d1za",
            "operating_organisation_id":"0190f7a2-7a1e-7c3e-9d2b-3c4d5e6f7a8b","market":"ZA","currency":"ZAR",
            "merchant_profile_reference":"pro_abc","provider_merchant_id":"merchant_d1",
            "provider_engine_instance_id":"hs-d1","certification_reference":"cert/d1",
            "merchant_activation_reference":"activation/d1","platform_account_ids":["pacct_buyer1"]{extra}}}]}}"#
        )
    }

    #[test]
    fn a_valid_binding_is_served_by_tenant() {
        let b = MerchantBindings::parse(&file("")).unwrap();
        let binding = b.get("tn_d1za").unwrap();
        assert!(binding.allows_platform_account("pacct_buyer1"));
        assert!(!binding.allows_platform_account("pacct_other"));
        assert!(b.get("tn_other").is_none());
    }

    #[test]
    fn unknown_fields_and_bad_values_refuse_the_file() {
        assert!(MerchantBindings::parse(&file(r#","provider_api_key":"x""#)).is_err());
        assert!(MerchantBindings::parse(&file("").replace("\"ZAR\"", "\"zar\"")).is_err());
        assert!(MerchantBindings::parse(&file("").replace("pacct_buyer1", "buyer1")).is_err());
        assert!(MerchantBindings::parse(r#"{"bindings":[]}"#).is_err());
        let doubled = file("").replace("}]}", "},") + &file("")[13..];
        assert!(MerchantBindings::parse(&doubled).is_err());
    }
}
