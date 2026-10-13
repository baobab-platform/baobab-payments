//! baobab-payments: the Baobab Payment API (ADR-PAY-0001) with the
//! non-production sandbox provider. The HyperSwitch transport exists but is not
//! enabled for payment execution until durable and governance acceptance.

pub mod auth;
pub mod config;
pub mod contracts;
pub mod cp_authority;
pub mod domain;
pub mod http;
pub mod hyperswitch;
pub mod native_execution;
pub mod provider;
pub mod service;
pub mod settlement_reconciliation;
pub mod webhook_reconciliation;

pub mod documentary_restriction;
pub mod legal_actor;
pub mod wire_json;
