use test_harness::{Harness, HttpErrorCode, LogEntry};

#[tokio::test(flavor = "multi_thread")]
async fn decides_by_method() -> wasmtime::Result<()> {
    let mut gate = Harness::new("gate")
        .latch("latch-method")
        .config("get", "deferred")
        .config("head", "")
        .config("*", "denied")
        .build()
        .await?;
    assert_eq!(
        gate.send(http::Method::GET, "https://example.com/")
            .await?
            .ok(),
        Some(200)
    );
    assert_eq!(
        gate.handle(http::Method::HEAD, "https://example.com/")
            .await?
            .ok(),
        Some(200)
    );
    let denied = gate
        .send(http::Method::POST, "https://example.com/")
        .await?;
    assert!(
        matches!(denied, Err(HttpErrorCode::HttpRequestMethodInvalid)),
        "{denied:?}"
    );
    // the method is matched in lower case
    let denied = gate
        .send(http::Method::from_bytes(b"PURGE")?, "https://example.com/")
        .await?;
    assert!(
        matches!(denied, Err(HttpErrorCode::HttpRequestMethodInvalid)),
        "{denied:?}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn defers_without_config() -> wasmtime::Result<()> {
    let mut gate = Harness::new("gate").latch("latch-method").build().await?;
    assert_eq!(
        gate.send(http::Method::DELETE, "https://example.com/")
            .await?
            .ok(),
        Some(200)
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_config_fails_every_request() -> wasmtime::Result<()> {
    let mut gate = Harness::new("gate")
        .latch("latch-method")
        .config("get", "deferred")
        .config("post", "maybe")
        .build()
        .await?;
    // even a request for a method with a valid value, the config is invalid as a whole
    let result = gate.send(http::Method::GET, "https://example.com/").await?;
    assert!(
        matches!(
            &result,
            Err(HttpErrorCode::InternalError(Some(message)))
                if message == "latch-error: invalid-config<latch-method>"
        ),
        "{result:?}"
    );
    assert_eq!(gate.recorder().requests(), vec![]);
    assert_eq!(
        gate.recorder().logs()[0],
        LogEntry::critical(
            "componentized-latch",
            "Invalid config LATCH=latch-method KEY=post VALUE=maybe ERROR=expected one of: 'deferred', 'denied'"
        )
    );
    Ok(())
}
