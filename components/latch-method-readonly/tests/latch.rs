//! Tests for `latch-method-readonly`, composed from `latch-method-readonly.wac`.

use test_harness::{Harness, HttpErrorCode};

#[tokio::test(flavor = "multi_thread")]
async fn allows_only_reading_methods() -> wasmtime::Result<()> {
    let mut gate = Harness::new("gate")
        .latch("latch-method-readonly")
        .build()
        .await?;
    for method in ["GET", "HEAD", "QUERY", "OPTIONS"] {
        let method = http::Method::from_bytes(method.as_bytes())?;
        assert_eq!(
            gate.send(method.clone(), "https://example.com/")
                .await?
                .ok(),
            Some(200),
            "{method}"
        );
    }
    for method in ["POST", "PUT", "PATCH", "DELETE", "BOGUS"] {
        let method = http::Method::from_bytes(method.as_bytes())?;
        let result = gate.send(method.clone(), "https://example.com/").await?;
        assert!(
            matches!(result, Err(HttpErrorCode::HttpRequestMethodInvalid)),
            "{method} {result:?}"
        );
    }
    Ok(())
}
