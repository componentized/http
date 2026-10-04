//! Tests for `gate`, composed from `gate.wac`. Tests for each interface live with `gate-client`
//! and `gate-handler`.

use test_harness::{Harness, HostLatch, HttpErrorCode};

#[tokio::test(flavor = "multi_thread")]
async fn gates_client_and_handler() -> wasmtime::Result<()> {
    let mut gate = Harness::new("gate")
        .host_latch(HostLatch::deny(&["handler.handle"]))
        .build()
        .await?;
    assert_eq!(
        gate.send(http::Method::GET, "https://example.com/")
            .await?
            .ok(),
        Some(200)
    );
    let handled = gate
        .handle(http::Method::GET, "https://example.com/")
        .await?;
    assert!(
        matches!(handled, Err(HttpErrorCode::HttpRequestDenied)),
        "{handled:?}"
    );
    assert_eq!(
        gate.recorder().operations(),
        vec!["client.send", "handler.handle"]
    );
    Ok(())
}
