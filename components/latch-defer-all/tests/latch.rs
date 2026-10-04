use test_harness::Harness;

#[tokio::test(flavor = "multi_thread")]
async fn defers_every_request() -> wasmtime::Result<()> {
    let mut gate = Harness::new("gate")
        .latch("latch-defer-all")
        .build()
        .await?;
    assert_eq!(
        gate.send(http::Method::POST, "https://example.com/")
            .await?
            .ok(),
        Some(200)
    );
    assert_eq!(
        gate.handle(http::Method::DELETE, "https://example.com/")
            .await?
            .ok(),
        Some(200)
    );
    // latch-defer-all decides every request itself, the host latch is never consulted
    assert_eq!(gate.recorder().operations(), Vec::<String>::new());
    assert_eq!(gate.recorder().logs(), vec![]);
    Ok(())
}
