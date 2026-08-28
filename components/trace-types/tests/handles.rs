//! The handle logged for a resource identifies it while debugging. The value depends on the
//! runtime, but must be the same in every message about the same resource, whatever name it is
//! logged under, e.g. `SELF` or `THIS`.

use test_harness::{ErrorCode, Harness, Method, Scheme, ready};

/// The single value logged for a resource, panics if it was not logged or logged with different
/// values.
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
async fn fields_handle_is_stable() -> wasmtime::Result<()> {
    let mut trace = Harness::new("trace-types").build().await?;
    trace
        .run(async |accessor, trace| {
            let fields = trace.wasi_http_types().fields();
            let a = fields.call_constructor(accessor).await?;
            let b = fields.call_constructor(accessor).await?;
            fields.call_has(accessor, a, "x".to_string()).await?;
            fields.call_has(accessor, b, "x".to_string()).await?;
            fields.call_copy_all(accessor, a).await?;
            fields.call_get(accessor, a, "x".to_string()).await?;
            fields.call_copy_all(accessor, b).await?;
            Ok(())
        })
        .await?;
    let values = trace.recorder().handle_values("wasi:http/types#fields.");
    let a = same_handle(
        vec![values[0].clone(), values[2].clone(), values[3].clone()],
        "fields",
    );
    let b = same_handle(vec![values[1].clone(), values[4].clone()], "fields");
    assert_ne!(a, b, "each fields has its own handle");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn request_handle_is_stable() -> wasmtime::Result<()> {
    let mut trace = Harness::new("trace-types").build().await?;
    trace
        .run(async |accessor, trace| {
            let request = trace
                .new_request(accessor, Method::Get, Scheme::Https, "example.com", "/")
                .await?;
            let requests = trace.wasi_http_types().request();
            requests.call_get_method(accessor, request).await?;
            requests.call_get_headers(accessor, request).await?;
            requests.call_get_options(accessor, request).await?;
            let res = ready(accessor, Ok::<(), ErrorCode>(()))?;
            requests.call_consume_body(accessor, request, res).await?;
            Ok(())
        })
        .await?;
    let handles = trace.recorder().handles();
    // logged as SELF by the methods and THIS by consume-body
    assert!(handles.iter().any(|handle| handle.key == "SELF"));
    assert!(handles.iter().any(|handle| handle.key == "THIS"));
    same_handle(
        trace.recorder().handle_values("wasi:http/types#request."),
        "request",
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn request_options_handle_is_stable() -> wasmtime::Result<()> {
    let mut trace = Harness::new("trace-types").build().await?;
    trace
        .run(async |accessor, trace| {
            let options = trace.wasi_http_types().request_options();
            let opts = options.call_constructor(accessor).await?;
            options
                .call_set_connect_timeout(accessor, opts, Some(1))
                .await?
                .expect("set-connect-timeout");
            options.call_get_connect_timeout(accessor, opts).await?;
            let clone = options.call_clone(accessor, opts).await?;
            options.call_get_first_byte_timeout(accessor, clone).await?;
            options
                .call_get_between_bytes_timeout(accessor, clone)
                .await?;
            options
                .call_get_between_bytes_timeout(accessor, opts)
                .await?;
            Ok(())
        })
        .await?;
    let values = trace
        .recorder()
        .handle_values("wasi:http/types#request-options.");
    // set, get, clone and the last get are on the original, the others on the clone
    let original = same_handle(
        vec![
            values[0].clone(),
            values[1].clone(),
            values[2].clone(),
            values[5].clone(),
        ],
        "request-options",
    );
    let clone = same_handle(
        vec![values[3].clone(), values[4].clone()],
        "request-options clone",
    );
    assert_ne!(original, clone, "the clone has its own handle");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn response_handle_is_stable() -> wasmtime::Result<()> {
    let mut trace = Harness::new("trace-types").build().await?;
    trace
        .run(async |accessor, trace| {
            let types = trace.wasi_http_types();
            let headers = types.fields().call_constructor(accessor).await?;
            let trailers = ready(accessor, Ok(None))?;
            let responses = types.response();
            let (response, _transmitted) = responses
                .call_new(accessor, headers, None, trailers)
                .await?;
            responses.call_get_status_code(accessor, response).await?;
            responses
                .call_set_status_code(accessor, response, 201)
                .await?
                .expect("set-status-code");
            responses.call_get_headers(accessor, response).await?;
            let res = ready(accessor, Ok::<(), ErrorCode>(()))?;
            responses.call_consume_body(accessor, response, res).await?;
            Ok(())
        })
        .await?;
    let handles = trace.recorder().handles();
    // logged as SELF by the methods and THIS by consume-body
    assert!(handles.iter().any(|handle| handle.key == "SELF"));
    assert!(handles.iter().any(|handle| handle.key == "THIS"));
    same_handle(
        trace.recorder().handle_values("wasi:http/types#response."),
        "response",
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn handles_are_hex() -> wasmtime::Result<()> {
    let mut trace = Harness::new("trace-types").build().await?;
    trace
        .run(async |accessor, trace| {
            let fields = trace.wasi_http_types().fields();
            // enough resources for handles past 9
            for _ in 0..32 {
                let f = fields.call_constructor(accessor).await?;
                fields.call_has(accessor, f, "x".to_string()).await?;
            }
            Ok(())
        })
        .await?;
    let values = trace.recorder().handle_values("wasi:http/types#fields.");
    assert_eq!(values.len(), 32);
    for value in &values {
        let digits = value
            .strip_prefix('&')
            .unwrap_or_else(|| panic!("prefixed with &: {value}"));
        assert_eq!(digits.len(), 4, "{value}");
        assert!(
            digits
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
            "lowercase hex: {value}"
        );
    }
    assert!(
        values
            .iter()
            .any(|value| value.bytes().any(|b| (b'a'..=b'f').contains(&b))),
        "handles past 9 use hex digits: {values:?}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn values_are_not_mistaken_for_handles() -> wasmtime::Result<()> {
    let mut trace = Harness::new("trace-types").build().await?;
    trace
        .run(async |accessor, trace| {
            let fields = trace.wasi_http_types().fields();
            let headers = fields.call_constructor(accessor).await?;
            // a name and value with the shape of a handle's digits
            fields
                .call_append(accessor, headers, "beef".to_string(), b"cafe".to_vec())
                .await?
                .expect("append");
            Ok(())
        })
        .await?;
    let handles = trace.recorder().handles();
    assert_eq!(handles.len(), 1, "{handles:?}");
    assert_eq!(handles[0].key, "SELF");
    let logs = trace.recorder().masked_logs();
    assert_eq!(
        logs.last().map(|log| log.message.as_str()),
        Some("OPERATION=wasi:http/types#fields.append SELF=&---- NAME=beef VALUE=cafe")
    );
    Ok(())
}
