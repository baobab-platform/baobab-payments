//! The implementation-neutral payment port (ADR-PAY-0001 section 15.2). The
//! HyperSwitch adapter will implement it; until then only the sandbox does.

use crate::domain::{Payment, PaymentIntent};

/// The outcome of authorising a payment method.
pub enum Authorization {
    Authorized { provider_reference: String },
    Declined { failure_code: &'static str },
}

pub trait PaymentProvider: Send + Sync {
    /// Shared `paymentProviderKind`.
    fn kind(&self) -> &'static str;
    /// True when no money moves and nothing settles.
    fn simulated(&self) -> bool;
    fn authorize(&self, intent: &PaymentIntent, payment_method_reference: &str) -> Authorization;
    fn capture(&self, payment: &Payment, amount_minor: i64);
    fn void(&self, payment: &Payment);
    /// Returns the provider's reference for the refund.
    fn refund(&self, payment: &Payment, amount_minor: i64) -> String;
    fn healthy(&self) -> bool;
}

/// The deterministic non-production sandbox (ADR-PAY-0001 section 15.2). It
/// contacts nothing, moves no money and marks every result simulated.
/// A payment method reference beginning `pm_sandbox_decline` is declined;
/// every other reference is authorised.
pub struct SandboxProvider;

impl PaymentProvider for SandboxProvider {
    fn kind(&self) -> &'static str {
        "SANDBOX"
    }

    fn simulated(&self) -> bool {
        true
    }

    fn authorize(&self, _intent: &PaymentIntent, payment_method_reference: &str) -> Authorization {
        if payment_method_reference.starts_with("pm_sandbox_decline") {
            Authorization::Declined {
                failure_code: "SANDBOX_DECLINED",
            }
        } else {
            Authorization::Authorized {
                provider_reference: crate::domain::new_id("sandbox"),
            }
        }
    }

    fn capture(&self, _payment: &Payment, _amount_minor: i64) {}

    fn void(&self, _payment: &Payment) {}

    fn refund(&self, _payment: &Payment, _amount_minor: i64) -> String {
        crate::domain::new_id("sandbox")
    }

    fn healthy(&self) -> bool {
        true
    }
}
