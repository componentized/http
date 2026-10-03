//! Tests for the `wasi:http/types` operations traced by every trace component, run against
//! `trace-types`.

use test_harness::{ErrorCode, Harness, LogEntry, Method, Scheme, ready};

fn traced(message: &str) -> LogEntry {
    LogEntry::trace("componentized-trace", message)
}

#[tokio::test(flavor = "multi_thread")]
async fn exports_only_types() -> wasmtime::Result<()> {
    let trace = Harness::new("trace-types").build().await?;
    assert!(trace.exports().exports_types());
    assert!(!trace.exports().exports_client());
    assert!(!trace.exports().exports_handler());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn fields_traced() -> wasmtime::Result<()> {
    let mut trace = Harness::new("trace-types").build().await?;
    let results = trace
        .run(async |accessor, trace| {
            let fields = trace.wasi_http_types().fields();
            let headers = fields
                .call_from_list(
                    accessor,
                    vec![("accept".to_string(), b"text/plain".to_vec())],
                )
                .await?
                .expect("from-list");
            fields
                .call_append(accessor, headers, "x-a".to_string(), b"1".to_vec())
                .await?
                .expect("append");
            fields
                .call_set(
                    accessor,
                    headers,
                    "x-b".to_string(),
                    vec![b"2".to_vec(), b"3".to_vec()],
                )
                .await?
                .expect("set");
            let get = fields
                .call_get(accessor, headers, "x-b".to_string())
                .await?;
            let has = fields
                .call_has(accessor, headers, "x-a".to_string())
                .await?;
            fields
                .call_delete(accessor, headers, "x-a".to_string())
                .await?
                .expect("delete");
            let deleted = fields
                .call_get_and_delete(accessor, headers, "x-b".to_string())
                .await?
                .expect("get-and-delete");
            let copy = fields.call_copy_all(accessor, headers).await?;
            let clone = fields.call_clone(accessor, headers).await?;
            let cloned = fields.call_copy_all(accessor, clone).await?;
            let empty = fields.call_constructor(accessor).await?;
            let empty = fields.call_copy_all(accessor, empty).await?;
            Ok((get, has, deleted, copy, cloned, empty))
        })
        .await?;
    let (get, has, deleted, copy, cloned, empty) = results;
    // the upstream values are passed through
    assert_eq!(get, vec![b"2".to_vec(), b"3".to_vec()]);
    assert!(has);
    assert_eq!(deleted, vec![b"2".to_vec(), b"3".to_vec()]);
    assert_eq!(copy, vec![("accept".to_string(), b"text/plain".to_vec())]);
    assert_eq!(cloned, copy);
    assert_eq!(empty, vec![]);
    assert_eq!(
        trace.recorder().masked_logs(),
        vec![
            traced("OPERATION=wasi:http/types#fields.from-list ENTRIES-LEN=1"),
            traced("OPERATION=wasi:http/types#fields.append SELF=&---- NAME=x-a VALUE=1"),
            traced("OPERATION=wasi:http/types#fields.set SELF=&---- NAME=x-b VALUE-LEN=2"),
            traced("OPERATION=wasi:http/types#fields.get SELF=&---- NAME=x-b"),
            traced("OPERATION=wasi:http/types#fields.has SELF=&---- NAME=x-a"),
            traced("OPERATION=wasi:http/types#fields.delete SELF=&---- NAME=x-a"),
            traced("OPERATION=wasi:http/types#fields.get-and-delete SELF=&---- NAME=x-b"),
            traced("OPERATION=wasi:http/types#fields.copy-all SELF=&----"),
            traced("OPERATION=wasi:http/types#fields.clone SELF=&----"),
            traced("OPERATION=wasi:http/types#fields.copy-all SELF=&----"),
            traced("OPERATION=wasi:http/types#fields.new"),
            traced("OPERATION=wasi:http/types#fields.copy-all SELF=&----"),
        ]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn fields_error_passed_through() -> wasmtime::Result<()> {
    let mut trace = Harness::new("trace-types").build().await?;
    let result = trace
        .run(async |accessor, trace| {
            let fields = trace.wasi_http_types().fields();
            let headers = fields.call_constructor(accessor).await?;
            Ok(fields
                .call_append(accessor, headers, "bad name".to_string(), b"1".to_vec())
                .await?)
        })
        .await?;
    assert!(result.is_err(), "invalid header name: {result:?}");
    assert_eq!(
        trace.recorder().masked_logs(),
        vec![
            traced("OPERATION=wasi:http/types#fields.new"),
            traced("OPERATION=wasi:http/types#fields.append SELF=&---- NAME=bad name VALUE=1"),
        ]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn request_traced() -> wasmtime::Result<()> {
    let mut trace = Harness::new("trace-types").build().await?;
    let results = trace
        .run(async |accessor, trace| {
            let request = trace
                .new_request(
                    accessor,
                    Method::Post,
                    Scheme::Http,
                    "example.com",
                    "/a?b=c",
                )
                .await?;
            let types = trace.wasi_http_types();
            let requests = types.request();
            let method = requests.call_get_method(accessor, request).await?;
            let scheme = requests.call_get_scheme(accessor, request).await?;
            let authority = requests.call_get_authority(accessor, request).await?;
            let path_with_query = requests.call_get_path_with_query(accessor, request).await?;
            let options = requests.call_get_options(accessor, request).await?;
            let headers = requests.call_get_headers(accessor, request).await?;
            let headers = types.fields().call_copy_all(accessor, headers).await?;
            let res = ready(accessor, Ok::<(), ErrorCode>(()))?;
            let (_contents, _trailers) = requests.call_consume_body(accessor, request, res).await?;
            Ok((
                method,
                scheme,
                authority,
                path_with_query,
                options.is_some(),
                headers,
            ))
        })
        .await?;
    let (method, scheme, authority, path_with_query, has_options, headers) = results;
    // the upstream values are passed through
    assert!(matches!(method, Method::Post), "{method:?}");
    assert!(matches!(scheme, Some(Scheme::Http)), "{scheme:?}");
    assert_eq!(authority.as_deref(), Some("example.com"));
    assert_eq!(path_with_query.as_deref(), Some("/a?b=c"));
    assert!(!has_options);
    assert_eq!(headers, vec![]);
    assert_eq!(
        trace.recorder().masked_logs(),
        vec![
            traced("OPERATION=wasi:http/types#fields.new"),
            traced("OPERATION=wasi:http/types#request.new"),
            traced("OPERATION=wasi:http/types#request.set-method SELF=&---- METHOD=post"),
            traced("OPERATION=wasi:http/types#request.set-scheme SELF=&---- SCHEME=some<http>"),
            traced(
                "OPERATION=wasi:http/types#request.set-authority SELF=&---- AUTHORITY=some<example.com>"
            ),
            traced(
                "OPERATION=wasi:http/types#request.set-path-with-query SELF=&---- PATH-WITH-QUERY=some</a?b=c>"
            ),
            traced("OPERATION=wasi:http/types#request.get-method SELF=&----"),
            traced("OPERATION=wasi:http/types#request.get-scheme SELF=&----"),
            traced("OPERATION=wasi:http/types#request.get-authority SELF=&----"),
            traced("OPERATION=wasi:http/types#request.get-path-with-query SELF=&----"),
            traced("OPERATION=wasi:http/types#request.get-options SELF=&----"),
            traced("OPERATION=wasi:http/types#request.get-headers SELF=&----"),
            traced("OPERATION=wasi:http/types#fields.copy-all SELF=&----"),
            traced("OPERATION=wasi:http/types#request.consume-body THIS=&----"),
        ]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn request_options_traced() -> wasmtime::Result<()> {
    let mut trace = Harness::new("trace-types").build().await?;
    let results = trace
        .run(async |accessor, trace| {
            let options = trace.wasi_http_types().request_options();
            let opts = options.call_constructor(accessor).await?;
            options
                .call_set_connect_timeout(accessor, opts, Some(1_000))
                .await?
                .expect("set-connect-timeout");
            options
                .call_set_first_byte_timeout(accessor, opts, Some(2_000))
                .await?
                .expect("set-first-byte-timeout");
            options
                .call_set_between_bytes_timeout(accessor, opts, None)
                .await?
                .expect("set-between-bytes-timeout");
            let clone = options.call_clone(accessor, opts).await?;
            let connect = options.call_get_connect_timeout(accessor, clone).await?;
            let first_byte = options.call_get_first_byte_timeout(accessor, clone).await?;
            let between_bytes = options
                .call_get_between_bytes_timeout(accessor, clone)
                .await?;
            Ok((connect, first_byte, between_bytes))
        })
        .await?;
    // the upstream values are passed through, including to the clone
    assert_eq!(results, (Some(1_000), Some(2_000), None));
    assert_eq!(
        trace.recorder().masked_logs(),
        vec![
            traced("OPERATION=wasi:http/types#request-options.new"),
            traced(
                "OPERATION=wasi:http/types#request-options.set-connect-timeout SELF=&---- DURATION=some<1000>"
            ),
            traced(
                "OPERATION=wasi:http/types#request-options.set-first-byte-timeout SELF=&---- DURATION=some<2000>"
            ),
            traced(
                "OPERATION=wasi:http/types#request-options.set-between-bytes-timeout SELF=&---- DURATION=none"
            ),
            traced("OPERATION=wasi:http/types#request-options.clone SELF=&----"),
            traced("OPERATION=wasi:http/types#request-options.get-connect-timeout SELF=&----"),
            traced("OPERATION=wasi:http/types#request-options.get-first-byte-timeout SELF=&----"),
            traced(
                "OPERATION=wasi:http/types#request-options.get-between-bytes-timeout SELF=&----"
            ),
        ]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn request_with_options_traced() -> wasmtime::Result<()> {
    let mut trace = Harness::new("trace-types").build().await?;
    let connect = trace
        .run(async |accessor, trace| {
            let types = trace.wasi_http_types();
            let options = types.request_options();
            let opts = options.call_constructor(accessor).await?;
            options
                .call_set_connect_timeout(accessor, opts, Some(1_000))
                .await?
                .expect("set-connect-timeout");
            let headers = types.fields().call_constructor(accessor).await?;
            let trailers = ready(accessor, Ok(None))?;
            let (request, _transmitted) = types
                .request()
                .call_new(accessor, headers, None, trailers, Some(opts))
                .await?;
            let opts = types
                .request()
                .call_get_options(accessor, request)
                .await?
                .expect("options");
            Ok(options.call_get_connect_timeout(accessor, opts).await?)
        })
        .await?;
    // the options are unwrapped for the upstream request, and wrapped again when read back
    assert_eq!(connect, Some(1_000));
    assert_eq!(
        trace.recorder().masked_logs(),
        vec![
            traced("OPERATION=wasi:http/types#request-options.new"),
            traced(
                "OPERATION=wasi:http/types#request-options.set-connect-timeout SELF=&---- DURATION=some<1000>"
            ),
            traced("OPERATION=wasi:http/types#fields.new"),
            traced("OPERATION=wasi:http/types#request.new"),
            traced("OPERATION=wasi:http/types#request.get-options SELF=&----"),
            traced("OPERATION=wasi:http/types#request-options.get-connect-timeout SELF=&----"),
        ]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn response_traced() -> wasmtime::Result<()> {
    let mut trace = Harness::new("trace-types").build().await?;
    let results = trace
        .run(async |accessor, trace| {
            let types = trace.wasi_http_types();
            let headers = types
                .fields()
                .call_from_list(accessor, vec![("x-a".to_string(), b"1".to_vec())])
                .await?
                .expect("from-list");
            let trailers = ready(accessor, Ok(None))?;
            let responses = types.response();
            let (response, _transmitted) = responses
                .call_new(accessor, headers, None, trailers)
                .await?;
            let default_status = responses.call_get_status_code(accessor, response).await?;
            responses
                .call_set_status_code(accessor, response, 404)
                .await?
                .expect("set-status-code");
            let status = responses.call_get_status_code(accessor, response).await?;
            let invalid = responses
                .call_set_status_code(accessor, response, 42)
                .await?;
            let headers = responses.call_get_headers(accessor, response).await?;
            let headers = types.fields().call_copy_all(accessor, headers).await?;
            let res = ready(accessor, Ok::<(), ErrorCode>(()))?;
            let (_contents, _trailers) =
                responses.call_consume_body(accessor, response, res).await?;
            Ok((default_status, status, invalid, headers))
        })
        .await?;
    let (default_status, status, invalid, headers) = results;
    // the upstream values and errors are passed through
    assert_eq!(default_status, 200);
    assert_eq!(status, 404);
    assert_eq!(invalid, Err(()));
    assert_eq!(headers, vec![("x-a".to_string(), b"1".to_vec())]);
    assert_eq!(
        trace.recorder().masked_logs(),
        vec![
            traced("OPERATION=wasi:http/types#fields.from-list ENTRIES-LEN=1"),
            traced("OPERATION=wasi:http/types#response.new"),
            traced("OPERATION=wasi:http/types#response.get-status-code SELF=&----"),
            traced("OPERATION=wasi:http/types#response.set-status-code SELF=&---- STATUS-CODE=404"),
            traced("OPERATION=wasi:http/types#response.get-status-code SELF=&----"),
            traced("OPERATION=wasi:http/types#response.set-status-code SELF=&---- STATUS-CODE=42"),
            traced("OPERATION=wasi:http/types#response.get-headers SELF=&----"),
            traced("OPERATION=wasi:http/types#fields.copy-all SELF=&----"),
            traced("OPERATION=wasi:http/types#response.consume-body THIS=&----"),
        ]
    );
    Ok(())
}
