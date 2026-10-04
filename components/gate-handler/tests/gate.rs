//! Tests for `gate-handler`, which asks a latch before handling each request.

use test_harness::{
    Authorization, Harness, HostLatch, HttpErrorCode, LogEntry, Observation, host_request,
    response_status,
};

const URI: &str = "https://example.com/items?page=2";

fn authorization() -> Authorization {
    Authorization {
        operation: "handler.handle".to_string(),
        method: "GET".to_string(),
        path_with_query: "/items?page=2".to_string(),
    }
}

fn observation(denied: bool) -> Observation {
    Observation {
        operation: "handler.handle".to_string(),
        denied,
    }
}

/// Handle a GET request through the gate, the status of the response or the error.
async fn send(
    gate: &mut test_harness::TestSubject,
) -> wasmtime::Result<Result<u16, HttpErrorCode>> {
    gate.run(async |accessor, gate| {
        let request = host_request(accessor, http::Method::GET, URI)?;
        Ok(
            match gate.gated_handler().call_handle(accessor, request).await? {
                Ok(response) => Ok(response_status(accessor, &response)?),
                Err(err) => Err(err),
            },
        )
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn exports_handler() -> wasmtime::Result<()> {
    let gate = Harness::new("gate-handler").build().await?;
    gate.exports().gated_handler();
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn deferred_request_is_handled() -> wasmtime::Result<()> {
    let mut gate = Harness::new("gate-handler").build().await?;
    let result = send(&mut gate).await?;
    assert_eq!(result.ok(), Some(200));
    assert_eq!(gate.recorder().authorizations(), vec![authorization()]);
    assert_eq!(gate.recorder().observations(), vec![observation(false)]);
    assert_eq!(gate.recorder().requests().len(), 1);
    assert_eq!(gate.recorder().logs(), vec![]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn denied_request_is_not_handled() -> wasmtime::Result<()> {
    let mut gate = Harness::new("gate-handler")
        .host_latch(HostLatch::deny(&["handler.handle"]))
        .build()
        .await?;
    let result = send(&mut gate).await?;
    assert!(
        matches!(result, Err(HttpErrorCode::HttpRequestDenied)),
        "{result:?}"
    );
    assert_eq!(gate.recorder().requests(), vec![]);
    // the latch observes the denial it decided
    assert_eq!(gate.recorder().observations(), vec![observation(true)]);
    assert_eq!(
        gate.recorder().logs(),
        vec![LogEntry::warn(
            "componentized-gate",
            "Denied REASON=http-request-denied OPERATION=wasi:http/handler#handle METHOD=get PATH-WITH-QUERY=some</items?page=2>"
        )]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn latch_error_fails_the_request() -> wasmtime::Result<()> {
    let mut gate = Harness::new("gate-handler")
        .host_latch(HostLatch::invalid_config("latch-example"))
        .build()
        .await?;
    let result = send(&mut gate).await?;
    assert!(
        matches!(
            &result,
            Err(HttpErrorCode::InternalError(Some(message)))
                if message == "latch-error: invalid-config<latch-example>"
        ),
        "{result:?}"
    );
    assert_eq!(gate.recorder().requests(), vec![]);
    // a decision that was never made is not observed
    assert_eq!(gate.recorder().observations(), vec![]);
    assert_eq!(
        gate.recorder().logs(),
        vec![LogEntry::error(
            "componentized-gate",
            "Latch error CODE=invalid-config<latch-example> OPERATION=wasi:http/handler#handle METHOD=get PATH-WITH-QUERY=some</items?page=2>"
        )]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn failed_observation_fails_the_request() -> wasmtime::Result<()> {
    let mut gate = Harness::new("gate-handler")
        .host_latch(HostLatch::defer().fail_observe())
        .build()
        .await?;
    let result = send(&mut gate).await?;
    assert!(
        matches!(
            &result,
            Err(HttpErrorCode::InternalError(Some(message)))
                if message == "latch-error: observation-failed<host>"
        ),
        "{result:?}"
    );
    // the latch could not act on the decision, the request is not handled
    assert_eq!(gate.recorder().requests(), vec![]);
    assert_eq!(gate.recorder().observations(), vec![observation(false)]);
    Ok(())
}
