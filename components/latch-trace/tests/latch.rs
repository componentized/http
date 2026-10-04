use test_harness::{Harness, HostLatch, HttpErrorCode, LogEntry, Observation};

#[tokio::test(flavor = "multi_thread")]
async fn traces_decisions_without_changing_them() -> wasmtime::Result<()> {
    let mut gate = Harness::new("gate")
        .latch("latch-trace")
        .host_latch(HostLatch::deny(&["client.send"]))
        .build()
        .await?;
    let sent = gate
        .send(http::Method::POST, "https://example.com/items")
        .await?;
    assert!(
        matches!(sent, Err(HttpErrorCode::HttpRequestDenied)),
        "{sent:?}"
    );
    assert_eq!(
        gate.handle(http::Method::GET, "https://example.com/")
            .await?
            .ok(),
        Some(200)
    );
    // the wrapped host latch decides and observes
    assert_eq!(
        gate.recorder().operations(),
        vec!["client.send", "handler.handle"]
    );
    assert_eq!(
        gate.recorder().observations(),
        vec![
            Observation {
                operation: "client.send".to_string(),
                denied: true
            },
            Observation {
                operation: "handler.handle".to_string(),
                denied: false
            },
        ]
    );
    let traces: Vec<LogEntry> = gate
        .recorder()
        .logs()
        .into_iter()
        .filter(|log| log.context == "componentized-latch")
        .collect();
    assert_eq!(
        traces,
        vec![
            LogEntry::trace(
                "componentized-latch",
                "Authorization DECISION=denied REASON=http-request-denied OPERATION=wasi:http/client#send METHOD=post PATH-WITH-QUERY=some</items>"
            ),
            LogEntry::trace(
                "componentized-latch",
                "Authorization DECISION=deferred OPERATION=wasi:http/handler#handle METHOD=get PATH-WITH-QUERY=some</>"
            ),
        ]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn traces_errors_without_changing_them() -> wasmtime::Result<()> {
    let mut gate = Harness::new("gate")
        .latch("latch-trace")
        .host_latch(HostLatch::invalid_config("latch-example"))
        .build()
        .await?;
    let sent = gate.send(http::Method::GET, "https://example.com/").await?;
    assert!(
        matches!(sent, Err(HttpErrorCode::InternalError(_))),
        "{sent:?}"
    );
    assert!(gate.recorder().logs().contains(&LogEntry::trace(
        "componentized-latch",
        "Authorization ERROR=invalid-config<latch-example> OPERATION=wasi:http/client#send METHOD=get PATH-WITH-QUERY=some</>"
    )));
    Ok(())
}
