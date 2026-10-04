use http_latch::{Decision, ErrorCode, HttpErrorCode, Latch, Operation};

struct DenyAllLatch {}

impl Latch for DenyAllLatch {
    fn authorize(_: Operation) -> Result<Decision, ErrorCode> {
        Ok(Decision::Denied(HttpErrorCode::HttpRequestDenied))
    }

    fn observe_decision(_final_decision: Decision, _operation: Operation) -> Result<(), ErrorCode> {
        // no side effects
        Ok(())
    }
}

http_latch::export!(DenyAllLatch with_types_in http_latch::bindings);
