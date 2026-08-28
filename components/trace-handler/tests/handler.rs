//! Tests for the `wasi:http/handler` operations traced by `trace-handler`.

use test_harness::{
    ErrorCode, Harness, LogEntry, Method, ResourceAny, Scheme, SentRequest, UPSTREAM_HEADER,
    UPSTREAM_STATUS,
};

fn traced(message: &str) -> LogEntry {
    LogEntry::trace("componentized-trace", message)
}

#[tokio::test(flavor = "multi_thread")]
async fn exports_types_and_handler() -> wasmtime::Result<()> {
    let trace = Harness::new("trace-handler").build().await?;
    assert!(trace.exports().exports_types());
    assert!(!trace.exports().exports_client());
    assert!(trace.exports().exports_handler());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn handle_traced() -> wasmtime::Result<()> {
    let mut trace = Harness::new("trace-handler").build().await?;
    let (status, header) = trace
        .run(async |accessor, trace| {
            let request = trace
                .new_request(
                    accessor,
                    Method::Get,
                    Scheme::Https,
                    "example.com",
                    "/hello",
                )
                .await?;
            let response: ResourceAny = trace
                .wasi_http_handler()
                .call_handle(accessor, request)
                .await?
                .expect("handle");
            // the response is a traced resource
            let types = trace.wasi_http_types();
            let status = types
                .response()
                .call_get_status_code(accessor, response)
                .await?;
            let headers = types
                .response()
                .call_get_headers(accessor, response)
                .await?;
            let header = types
                .fields()
                .call_get(accessor, headers, UPSTREAM_HEADER.0.to_string())
                .await?;
            Ok((status, header))
        })
        .await?;
    // the upstream response is passed through
    assert_eq!(status, UPSTREAM_STATUS);
    assert_eq!(header, vec![UPSTREAM_HEADER.1.as_bytes().to_vec()]);
    assert_eq!(
        trace.recorder().requests(),
        vec![SentRequest {
            method: "GET".to_string(),
            uri: "https://example.com/hello".to_string(),
        }]
    );
    assert_eq!(
        trace.recorder().masked_logs(),
        vec![
            traced("OPERATION=wasi:http/types#fields.new"),
            traced("OPERATION=wasi:http/types#request.new"),
            traced("OPERATION=wasi:http/types#request.set-method SELF=&---- METHOD=get"),
            traced("OPERATION=wasi:http/types#request.set-scheme SELF=&---- SCHEME=some<https>"),
            traced(
                "OPERATION=wasi:http/types#request.set-authority SELF=&---- AUTHORITY=some<example.com>"
            ),
            traced(
                "OPERATION=wasi:http/types#request.set-path-with-query SELF=&---- PATH-WITH-QUERY=some</hello>"
            ),
            traced("OPERATION=wasi:http/handler#handle REQUEST=&----"),
            traced("OPERATION=wasi:http/types#response.get-status-code SELF=&----"),
            traced("OPERATION=wasi:http/types#response.get-headers SELF=&----"),
            traced("OPERATION=wasi:http/types#fields.get SELF=&---- NAME=x-upstream"),
        ]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn handle_error_passed_through() -> wasmtime::Result<()> {
    let mut trace = Harness::new("trace-handler").build().await?;
    let result = trace
        .run(async |accessor, trace| {
            let scheme = Scheme::Other("gopher".to_string());
            let request = trace
                .new_request(accessor, Method::Get, scheme, "example.com", "/")
                .await?;
            let result = trace
                .wasi_http_handler()
                .call_handle(accessor, request)
                .await?;
            Ok(result.map(|_| ()))
        })
        .await?;
    // the upstream error is passed through, the request never left
    assert!(
        matches!(result, Err(ErrorCode::HttpProtocolError)),
        "unsupported scheme: {result:?}"
    );
    assert_eq!(trace.recorder().requests(), vec![]);
    assert_eq!(
        trace.recorder().masked_logs().last(),
        Some(&traced("OPERATION=wasi:http/handler#handle REQUEST=&----"))
    );
    Ok(())
}
