use http_latch::{Decision, ErrorCode, Latch, Operation};

struct DeferAllLatch {}

impl Latch for DeferAllLatch {
    fn authorize(_: Operation) -> Result<Decision, ErrorCode> {
        Ok(Decision::Deferred)
    }

    fn observe_decision(_final_decision: Decision, _operation: Operation) -> Result<(), ErrorCode> {
        // no side effects
        Ok(())
    }
}

http_latch::export!(DeferAllLatch with_types_in http_latch::bindings);
