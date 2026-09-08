//! Tests for `latch-scheme-httpsonly`, composed from `latch-scheme-httpsonly.wac`.

use test_harness::{Harness, HttpErrorCode};

#[tokio::test(flavor = "multi_thread")]
async fn allows_only_https() -> wasmtime::Result<()> {
    let mut gate = Harness::new("gate")
        .latch("latch-scheme-httpsonly")
        .build()
        .await?;
    assert_eq!(
        gate.send(http::Method::GET, "https://example.com/")
            .await?
            .ok(),
        Some(200)
    );
    let result = gate.send(http::Method::GET, "http://example.com/").await?;
    assert!(
        matches!(result, Err(HttpErrorCode::HttpRequestDenied)),
        "{result:?}"
    );
    Ok(())
}
