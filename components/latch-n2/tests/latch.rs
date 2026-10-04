use test_harness::{Harness, HostLatch, HttpErrorCode, Observation};

#[tokio::test(flavor = "multi_thread")]
async fn any_latch_denies() -> wasmtime::Result<()> {
    // the latches, with the host latch last
    let mut gate = Harness::new("gate")
        .latch("latch-defer-all")
        .latch("latch-defer-all")
        .latch("latch-deny-all")
        .host_latch(HostLatch::defer())
        .build()
        .await?;
    let sent = gate.send(http::Method::GET, "https://example.com/").await?;
    assert!(
        matches!(sent, Err(HttpErrorCode::HttpRequestDenied)),
        "{sent:?}"
    );
    // the host latch after the denial is not asked, but observes the final decision
    assert_eq!(gate.recorder().operations(), Vec::<String>::new());
    assert_eq!(
        gate.recorder().observations(),
        vec![Observation {
            operation: "client.send".to_string(),
            denied: true
        }]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn defers_when_every_latch_defers() -> wasmtime::Result<()> {
    let mut gate = Harness::new("gate")
        .latch("latch-defer-all")
        .latch("latch-defer-all")
        .latch("latch-defer-all")
        .host_latch(HostLatch::defer())
        .build()
        .await?;
    assert_eq!(
        gate.send(http::Method::GET, "https://example.com/")
            .await?
            .ok(),
        Some(200)
    );
    assert_eq!(gate.recorder().operations(), vec!["client.send"]);
    Ok(())
}
