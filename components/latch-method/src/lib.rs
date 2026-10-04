use http_latch::{Decision, DecisionConfig, ErrorCode, HttpErrorCode, Latch, Operation};

const LATCH_NAME: &str = "latch-method";

struct MethodLatch {}

impl Latch for MethodLatch {
    fn authorize(operation: Operation) -> Result<Decision, ErrorCode> {
        let config = http_latch::load_config(LATCH_NAME, DecisionConfig::parse)?;
        let method = http_latch::request(&operation).get_method().to_string();
        Ok(config.decide(&method, || HttpErrorCode::HttpRequestMethodInvalid))
    }

    fn observe_decision(_final_decision: Decision, _operation: Operation) -> Result<(), ErrorCode> {
        // no side effects
        Ok(())
    }
}

http_latch::export!(MethodLatch with_types_in http_latch::bindings);
