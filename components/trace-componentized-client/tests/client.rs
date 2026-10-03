//! Tests for the `componentized:http/client` operations traced by `trace-componentized-client`.

use test_harness::{
    Harness, HttpClientMethod, LogEntry, SentRequest, UPSTREAM_HEADER, UPSTREAM_STATUS,
};

fn traced(message: &str) -> LogEntry {
    LogEntry::trace("componentized-trace", message)
}

#[tokio::test(flavor = "multi_thread")]
async fn exports_only_http_client() -> wasmtime::Result<()> {
    let trace = Harness::new("trace-componentized-client").build().await?;
    assert!(!trace.exports().exports_types());
    assert!(!trace.exports().exports_client());
    assert!(!trace.exports().exports_handler());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn request_traced() -> wasmtime::Result<()> {
    let mut trace = Harness::new("trace-componentized-client").build().await?;
    let (status, headers) = trace
        .run(async |accessor, trace| {
            let response = trace
                .componentized_http_client()
                .call_request(
                    accessor,
                    HttpClientMethod::Delete,
                    "https://example.com/a".to_string(),
                    vec![],
                    None,
                    None,
                )
                .await?
                .expect("request");
            Ok((response.status, response.headers))
        })
        .await?;
    // the upstream response is passed through
    assert_eq!(status, UPSTREAM_STATUS);
    assert_eq!(
        headers,
        vec![(UPSTREAM_HEADER.0.to_string(), UPSTREAM_HEADER.1.to_string())]
    );
    assert_eq!(
        trace.recorder().requests(),
        vec![SentRequest {
            method: "DELETE".to_string(),
            uri: "https://example.com/a".to_string(),
        }]
    );
    assert_eq!(
        trace.recorder().logs(),
        vec![traced(
            "OPERATION=componentized:http/client#request URL=https://example.com/a"
        )]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn methods_traced() -> wasmtime::Result<()> {
    let mut trace = Harness::new("trace-componentized-client").build().await?;
    trace
        .run(async |accessor, trace| {
            let client = trace.componentized_http_client();
            let url = |path: &str| format!("https://example.com/{path}");
            client
                .call_get(accessor, url("get"), vec![], None)
                .await?
                .expect("get");
            client
                .call_delete(accessor, url("delete"), vec![], None)
                .await?
                .expect("delete");
            client
                .call_head(accessor, url("head"), vec![], None)
                .await?
                .expect("head");
            client
                .call_options(accessor, url("options"), vec![], None)
                .await?
                .expect("options");
            client
                .call_trace(accessor, url("trace"), vec![], None)
                .await?
                .expect("trace");
            Ok(())
        })
        .await?;
    let methods: Vec<String> = trace
        .recorder()
        .requests()
        .into_iter()
        .map(|request| request.method)
        .collect();
    assert_eq!(methods, vec!["GET", "DELETE", "HEAD", "OPTIONS", "TRACE"]);
    assert_eq!(
        trace.recorder().logs(),
        vec![
            traced("OPERATION=componentized:http/client#get URL=https://example.com/get"),
            traced("OPERATION=componentized:http/client#delete URL=https://example.com/delete"),
            traced("OPERATION=componentized:http/client#head URL=https://example.com/head"),
            traced("OPERATION=componentized:http/client#options URL=https://example.com/options"),
            traced("OPERATION=componentized:http/client#trace URL=https://example.com/trace"),
        ]
    );
    Ok(())
}
