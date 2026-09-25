//! The vendored contracts are byte-for-byte the Shared commit
//! contracts.lock.yaml pins. SHARED_CONTRACTS_DIR names a baobab-platform/shared
//! checkout at that commit (CI checks it out); without one the test only checks
//! the lock declares every embedded file.

use baobab_payments::contracts::EMBEDDED;

#[test]
fn vendored_contracts_match_the_pinned_shared_commit() {
    let lock = std::fs::read_to_string("contracts.lock.yaml").expect("contracts.lock.yaml");
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
