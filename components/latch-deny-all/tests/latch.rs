use test_harness::{Harness, HttpErrorCode, LogEntry};

#[tokio::test(flavor = "multi_thread")]
async fn denies_every_request() -> wasmtime::Result<()> {
    let mut gate = Harness::new("gate").latch("latch-deny-all").build().await?;
    let sent = gate.send(http::Method::GET, "https://example.com/").await?;
    assert!(
        matches!(sent, Err(HttpErrorCode::HttpRequestDenied)),
        "{sent:?}"
    );
    let handled = gate
        .handle(http::Method::GET, "https://example.com/")
        .await?;
    assert!(
        matches!(handled, Err(HttpErrorCode::HttpRequestDenied)),
        "{handled:?}"
    );
    assert_eq!(gate.recorder().requests(), vec![]);
    assert_eq!(gate.recorder().operations(), Vec::<String>::new());
    assert_eq!(
        gate.recorder().logs()[0],
        LogEntry::warn(
            "componentized-gate",
            "Denied REASON=http-request-denied OPERATION=wasi:http/client#send METHOD=get PATH-WITH-QUERY=some</>"
        )
    );
    Ok(())
}
