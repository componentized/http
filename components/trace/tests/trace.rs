//! Tests for `trace`, which traces `wasi:http/types`, `wasi:http/client` and `wasi:http/handler`.
//! Tests for the individual operations live with `trace-types`, `trace-client` and
//! `trace-handler`, which are built from the same sources.

use test_harness::{Harness, LogEntry, Method, Scheme, SentRequest, UPSTREAM_STATUS};

fn traced(message: &str) -> LogEntry {
    LogEntry::trace("componentized-trace", message)
}

#[tokio::test(flavor = "multi_thread")]
async fn exports_types_client_and_handler() -> wasmtime::Result<()> {
    let trace = Harness::new("trace").build().await?;
    assert!(trace.exports().exports_types());
    assert!(trace.exports().exports_client());
    assert!(trace.exports().exports_handler());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn send_and_handle_traced() -> wasmtime::Result<()> {
    let mut trace = Harness::new("trace").build().await?;
    let statuses = trace
        .run(async |accessor, trace| {
            let types = trace.wasi_http_types();

            let request = trace
                .new_request(
                    accessor,
                    Method::Get,
                    Scheme::Https,
                    "example.com",
                    "/client",
                )
                .await?;
            let response = trace
                .wasi_http_client()
                .call_send(accessor, request)
                .await?
                .expect("send");
            let sent = types
                .response()
                .call_get_status_code(accessor, response)
                .await?;

            let request = trace
                .new_request(
                    accessor,
                    Method::Put,
                    Scheme::Https,
                    "example.com",
                    "/handler",
                )
                .await?;
            let response = trace
                .wasi_http_handler()
                .call_handle(accessor, request)
                .await?
                .expect("handle");
            let handled = types
                .response()
                .call_get_status_code(accessor, response)
                .await?;

            Ok((sent, handled))
        })
        .await?;
    assert_eq!(statuses, (UPSTREAM_STATUS, UPSTREAM_STATUS));
    assert_eq!(
        trace.recorder().requests(),
        vec![
            SentRequest {
                method: "GET".to_string(),
                uri: "https://example.com/client".to_string(),
            },
            SentRequest {
                method: "PUT".to_string(),
                uri: "https://example.com/handler".to_string(),
            },
        ]
    );
    let logs = trace.recorder().masked_logs();
    // types, client and handler are all traced
    assert!(logs.contains(&traced("OPERATION=wasi:http/types#request.new")));
    assert!(logs.contains(&traced("OPERATION=wasi:http/client#send REQUEST=&----")));
    assert!(logs.contains(&traced("OPERATION=wasi:http/handler#handle REQUEST=&----")));
    assert_eq!(
        logs.iter()
            .filter(|log| log.message.contains("response.get-status-code"))
            .count(),
        2
    );
    Ok(())
}
