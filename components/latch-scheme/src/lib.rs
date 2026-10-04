use http_latch::{Decision, DecisionConfig, ErrorCode, HttpErrorCode, Latch, Operation};

const LATCH_NAME: &str = "latch-scheme";

/// The key for a request without a scheme.
const NO_SCHEME: &str = "_";

struct SchemeLatch {}

impl Latch for SchemeLatch {
    fn authorize(operation: Operation) -> Result<Decision, ErrorCode> {
        let config = http_latch::load_config(LATCH_NAME, DecisionConfig::parse)?;
        let scheme = match http_latch::request(&operation).get_scheme() {
            Some(scheme) => scheme.to_string(),
            None => NO_SCHEME.to_string(),
        };
        Ok(config.decide(&scheme, || HttpErrorCode::HttpRequestDenied))
    }

    fn observe_decision(_final_decision: Decision, _operation: Operation) -> Result<(), ErrorCode> {
        // no side effects
        Ok(())
    }
}

http_latch::export!(SchemeLatch with_types_in http_latch::bindings);
