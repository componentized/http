use test_harness::{Harness, HostLatch, LogEntry, Observation};

#[tokio::test(flavor = "multi_thread")]
async fn logs_denials_without_enforcing_them() -> wasmtime::Result<()> {
    let mut gate = Harness::new("gate")
        .latch("latch-dry-run")
        .host_latch(HostLatch::deny(&["client.send"]))
        .build()
        .await?;
    // sent, though the wrapped latch denies it
    assert_eq!(
        gate.send(http::Method::POST, "https://example.com/items")
            .await?
            .ok(),
        Some(200)
    );
    assert_eq!(gate.recorder().requests().len(), 1);
    // the wrapped latch observes what actually happened
    assert_eq!(
        gate.recorder().observations(),
        vec![Observation {
            operation: "client.send".to_string(),
            denied: false
        }]
    );
    assert_eq!(
        gate.recorder().logs(),
        vec![LogEntry::warn(
            "componentized-latch",
            "Dry run, would deny REASON=http-request-denied OPERATION=wasi:http/client#send METHOD=post PATH-WITH-QUERY=some</items>"
        )]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn logs_errors_without_failing() -> wasmtime::Result<()> {
    let mut gate = Harness::new("gate")
        .latch("latch-dry-run")
        .host_latch(HostLatch::invalid_config("latch-example").fail_observe())
        .build()
        .await?;
    assert_eq!(
        gate.send(http::Method::GET, "https://example.com/")
            .await?
            .ok(),
        Some(200)
    );
    assert_eq!(
        gate.recorder().logs(),
        vec![
            LogEntry::error(
                "componentized-latch",
                "Dry run, latch error CODE=invalid-config<latch-example> OPERATION=wasi:http/client#send METHOD=get PATH-WITH-QUERY=some</>"
            ),
            LogEntry::error(
                "componentized-latch",
                "Dry run, latch error CODE=observation-failed<host> OPERATION=wasi:http/client#send METHOD=get PATH-WITH-QUERY=some</>"
            ),
        ]
    );
    Ok(())
}
