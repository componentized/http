use http_latch_n::{Decision, ErrorCode, Latch, Operation};
use http_latch_n::{latch0, latch1};

struct LatchN2 {}

impl Latch for LatchN2 {
    fn authorize(operation: Operation<'_>) -> Result<Decision, ErrorCode> {
        let authorizers = vec![latch0::authorize, latch1::authorize];
        http_latch_n::authorize(operation, authorizers)
    }

    fn observe_decision(
        final_decision: Decision,
        operation: Operation<'_>,
    ) -> Result<(), ErrorCode> {
        let observers = vec![latch0::observe_decision, latch1::observe_decision];
        http_latch_n::observe_decision(final_decision, operation, observers)
    }
}

http_latch_n::export!(LatchN2 with_types_in http_latch_n::bindings);
