//! P0-04B Payments: which execution path a public mutation may take. Activation
//! is decided per environment and per route here, in code reviewed with each
//! change; it is never a flag a deployment can set. Staging, production and any
//! unrecognised environment refuse every financial mutation with 503 until an
//! explicitly approved activation change lands. A payout has no public route,
//! and `money_movement` refuses it before any I/O wherever it is asked for.

/// The public mutations of the Payment API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    CreateIntent,
    ConfirmIntent,
    CancelIntent,
    Capture,
    Refund,
}

impl Route {
    pub const ALL: [Self; 5] = [
        Self::CreateIntent,
        Self::ConfirmIntent,
        Self::CancelIntent,
        Self::Capture,
        Self::Refund,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionPath {
    /// The in-memory simulated provider: moves no money.
    Sandbox,
    /// The money-movement enforcement point: CP authority, Payments' own
    /// approval, the durable journal and the provider.
    Native,
    /// 503 PAYMENT_EXECUTION_NOT_CERTIFIED.
    Refused,
}

/// The path for one route. `native_backend` is whether the native backend
/// was configured, which configuration accepts only in development and
/// integration; it can never open a protected environment.
pub fn execution_path(environment: &str, native_backend: bool, route: Route) -> ExecutionPath {
    // Every route has the same answer today; the route is part of the
    // signature so that activating one operation is a reviewed change here.
    match (environment, route) {
        ("development" | "integration", _) if native_backend => ExecutionPath::Native,
        ("development" | "integration", _) => ExecutionPath::Sandbox,
        _ => ExecutionPath::Refused,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_environments_refuse_every_route_even_with_a_native_backend() {
        for environment in ["staging", "production", "Production", "sandbox", "test", ""] {
            for native in [false, true] {
                for route in Route::ALL {
                    assert_eq!(
                        execution_path(environment, native, route),
                        ExecutionPath::Refused,
                        "{environment:?} {native} {route:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn local_environments_use_the_native_backend_only_when_configured() {
        for environment in ["development", "integration"] {
            for route in Route::ALL {
                assert_eq!(
                    execution_path(environment, false, route),
                    ExecutionPath::Sandbox
                );
                assert_eq!(
                    execution_path(environment, true, route),
                    ExecutionPath::Native
                );
            }
        }
    }
}
