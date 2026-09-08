use http_latch::{
    Decision, DisplayError, DisplayReason, ErrorCode, Latch, Operation, wrapped as latch,
};

struct DryRunLatch {}

impl Latch for DryRunLatch {
    fn authorize(operation: Operation) -> Result<Decision, ErrorCode> {
        // the wrapped latch decides, its denials and errors are logged but never enforced
        match latch::authorize(&operation) {
            Ok(Decision::Deferred) => {}
            Ok(Decision::Denied(reason)) => {
                http_latch::warn!(
                    "Dry run, would deny REASON={} {operation}",
                    DisplayReason(&reason)
                );
            }
            Err(err) => {
                http_latch::error!(
                    "Dry run, latch error CODE={} {operation}",
                    DisplayError(&err)
                );
            }
        }
        Ok(Decision::Deferred)
    }

    fn observe_decision(final_decision: Decision, operation: Operation) -> Result<(), ErrorCode> {
        // the wrapped latch observes what actually happens, the request was not denied by this
        // latch, so stateful latches act as if their denials were not enforced
        if let Err(err) = latch::observe_decision(&final_decision, &operation) {
            http_latch::error!(
                "Dry run, latch error CODE={} {operation}",
                DisplayError(&err)
            );
        }
        Ok(())
    }
}

http_latch::export!(DryRunLatch with_types_in http_latch::bindings);
