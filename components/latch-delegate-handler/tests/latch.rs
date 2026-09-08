use test_harness::{Harness, HostLatch, HttpErrorCode, Observation};

#[tokio::test(flavor = "multi_thread")]
async fn delegates_only_handler_requests() -> wasmtime::Result<()> {
    let mut gate = Harness::new("gate")
        .latch("latch-delegate-handler")
        .host_latch(HostLatch::deny(&["client.send", "handler.handle"]))
        .build()
        .await?;
    let sent = gate.send(http::Method::GET, "https://example.com/").await?;
    let handled = gate
        .handle(http::Method::GET, "https://example.com/")
        .await?;
    let (delegated, other) = match "handler" {
        "client" => (sent, handled),
        _ => (handled, sent),
    };
    // the wrapped latch denies what it is asked about, the rest is deferred without asking it
    assert!(
        matches!(delegated, Err(HttpErrorCode::HttpRequestDenied)),
        "{delegated:?}"
    );
    assert_eq!(other.ok(), Some(200));
    assert_eq!(gate.recorder().operations(), vec!["handler.handle"]);
    assert_eq!(
        gate.recorder().observations(),
        vec![Observation {
            operation: "handler.handle".to_string(),
            denied: true
        }]
    );
    Ok(())
}
