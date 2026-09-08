use http_latch::wrapped as latch;
use http_latch::{Decision, ErrorCode, Latch, Operation};

struct DelegateClientLatch {}

/// Only wasi:http/client requests reach the wrapped latch.
fn is_delegated(operation: &Operation) -> bool {
    matches!(operation, Operation::Client(_))
}

impl Latch for DelegateClientLatch {
    fn authorize(operation: Operation) -> Result<Decision, ErrorCode> {
        match is_delegated(&operation) {
            true => latch::authorize(&operation),
            false => Ok(Decision::Deferred),
        }
    }

    fn observe_decision(final_decision: Decision, operation: Operation) -> Result<(), ErrorCode> {
        // the wrapped latch only observes the requests it was asked to authorize
        match is_delegated(&operation) {
            true => latch::observe_decision(&final_decision, &operation),
            false => Ok(()),
        }
    }
}

http_latch::export!(DelegateClientLatch with_types_in http_latch::bindings);
