use test_harness::{Harness, HttpErrorCode, LogEntry};

#[tokio::test(flavor = "multi_thread")]
async fn decides_by_scheme() -> wasmtime::Result<()> {
    let mut gate = Harness::new("gate")
        .latch("latch-scheme")
        .config("https", "deferred")
        .config("*", "denied")
        .build()
        .await?;
    assert_eq!(
        gate.send(http::Method::GET, "https://example.com/")
            .await?
            .ok(),
        Some(200)
    );
    let denied = gate.send(http::Method::GET, "http://example.com/").await?;
    assert!(
        matches!(denied, Err(HttpErrorCode::HttpRequestDenied)),
        "{denied:?}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_config_fails_every_request() -> wasmtime::Result<()> {
    let mut gate = Harness::new("gate")
        .latch("latch-scheme")
        .config("https", "allowed")
        .build()
        .await?;
    let result = gate.send(http::Method::GET, "https://example.com/").await?;
    assert!(
        matches!(
            &result,
            Err(HttpErrorCode::InternalError(Some(message)))
                if message == "latch-error: invalid-config<latch-scheme>"
        ),
        "{result:?}"
    );
    assert_eq!(
        gate.recorder().logs()[0],
        LogEntry::critical(
            "componentized-latch",
            "Invalid config LATCH=latch-scheme KEY=https VALUE=allowed ERROR=expected one of: 'deferred', 'denied'"
        )
    );
    Ok(())
}
