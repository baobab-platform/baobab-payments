//! The vendored contracts are byte-for-byte the Shared commit
//! contracts.lock.yaml pins. SHARED_CONTRACTS_DIR names a baobab-platform/shared
//! checkout at that commit (CI checks it out); without one the test only checks
//! the lock declares every embedded file.

use baobab_payments::contracts::{EMBEDDED, SHARED_COMMIT};

#[test]
fn vendored_contracts_match_the_pinned_shared_commit() {
    let lock = std::fs::read_to_string("contracts.lock.yaml").expect("contracts.lock.yaml");
    assert!(lock.contains(&format!("  commit: {SHARED_COMMIT}\n")));
    let declared = lock.lines().filter(|line| line.starts_with("  - ")).count();
    assert_eq!(
        declared,
        EMBEDDED.len(),
        "lock has no omitted embedded paths"
    );
    if let Ok(shared) = std::env::var("SHARED_CONTRACTS_DIR") {
        let head = std::process::Command::new("git")
            .args(["-C", &shared, "rev-parse", "HEAD"])
            .output()
            .expect("immutable Shared checkout");
        assert!(head.status.success());
        assert_eq!(
            String::from_utf8(head.stdout).unwrap().trim(),
            SHARED_COMMIT
        );
    }
    for (path, text) in EMBEDDED {
        assert!(
            lock.contains(&format!("  - contracts/{path}\n")),
            "{path} is embedded but not declared in contracts.lock.yaml"
        );
        if let Ok(shared) = std::env::var("SHARED_CONTRACTS_DIR") {
            let pinned = std::fs::read_to_string(format!("{shared}/contracts/{path}"))
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            assert_eq!(
                &pinned, text,
                "{path} differs from the pinned Shared commit"
            );
        }
    }
}

#[test]
fn all_authority_wires_match_the_synthetic_owner_golden_matrix() {
    baobab_payments::contracts::self_test();
    let count = baobab_payments::contracts::check_conformance(include_str!(
        "fixtures/authority-contract-cases.json"
    ))
    .unwrap();
    assert!(count > 100);
}

#[test]
fn contract_errors_do_not_return_sensitive_instance_values() {
    let sensitive = serde_json::json!({"payment_method_ref":"synthetic-secret-never-echo"});
    let errors = baobab_payments::contracts::problems(
        &baobab_payments::contracts::def(
            baobab_payments::contracts::PAYMENT,
            "CreatePaymentIntentRequest",
        ),
        &sensitive,
    );
    assert!(!errors.is_empty());
    assert!(
        errors
            .iter()
            .all(|error| !error.contains("synthetic-secret-never-echo"))
    );
}
