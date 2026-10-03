//! The handle logged for a resource must be the same in every message about it, including when
//! it is passed to `wasi:http/handler#handle`.

use test_harness::{ErrorCode, Harness, Method, Scheme, ready};

fn same_handle(values: Vec<String>, resource: &str) -> String {
    let first = values
        .first()
        .unwrap_or_else(|| panic!("no handle logged for the {resource}"))
        .clone();
    assert!(
        values.iter().all(|value| *value == first),
        "the {resource} was logged with different handles: {values:?}"
    );
    first
}

#[tokio::test(flavor = "multi_thread")]
async fn handle_logs_the_request_handle() -> wasmtime::Result<()> {
    let mut trace = Harness::new("trace-handler").build().await?;
    trace
        .run(async |accessor, trace| {
            let request = trace
                .new_request(accessor, Method::Get, Scheme::Https, "example.com", "/")
                .await?;
            let response = trace
                .wasi_http_handler()
                .call_handle(accessor, request)
                .await?
                .expect("handle");
            let responses = trace.wasi_http_types().response();
            responses.call_get_status_code(accessor, response).await?;
            responses.call_get_headers(accessor, response).await?;
            let res = ready(accessor, Ok::<(), ErrorCode>(()))?;
            responses.call_consume_body(accessor, response, res).await?;
            Ok(())
        })
        .await?;
    let recorder = trace.recorder();
    let request = same_handle(
        recorder.handle_values("wasi:http/types#request."),
        "request",
    );
    assert_eq!(
        recorder.handle_values("wasi:http/handler#handle"),
        vec![request],
        "handle logs the handle of the request it was passed"
    );
    // the response from upstream is a new resource, its handle is stable too
    same_handle(
        recorder.handle_values("wasi:http/types#response."),
        "response",
    );
    Ok(())
}
