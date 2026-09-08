use http_latch::{
    Decision, DisplayDecision, DisplayError, ErrorCode, Latch, Operation, wrapped as latch,
};

struct TraceLatch {}

impl Latch for TraceLatch {
    fn authorize(operation: Operation) -> Result<Decision, ErrorCode> {
        // the wrapped latch decides, its decision is logged and returned unchanged
        let result = latch::authorize(&operation);
        match &result {
            Ok(decision) => {
                http_latch::trace!("Authorization {} {operation}", DisplayDecision(decision));
            }
            Err(err) => {
                http_latch::trace!("Authorization ERROR={} {operation}", DisplayError(err));
            }
        }
        result
    }

    fn observe_decision(final_decision: Decision, operation: Operation) -> Result<(), ErrorCode> {
        latch::observe_decision(&final_decision, &operation)
    }
}

http_latch::export!(TraceLatch with_types_in http_latch::bindings);
