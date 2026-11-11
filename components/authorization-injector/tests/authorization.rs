//! Tests for `authorization-injector`, composed from `authorization-injector.wac`.

use test_harness::{Harness, host_request_with, response_status};

#[tokio::test(flavor = "multi_thread")]
async fn adds_authorization_to_every_standard_method_but_trace() -> wasmtime::Result<()> {
    let mut client = Harness::new("authorization-injector")
        .config("https://api.example.com", "Bearer secret")
        .build()
        .await?;
    let cases = [
        ("GET", true),
        ("HEAD", true),
        ("QUERY", true),
        ("OPTIONS", true),
        ("POST", true),
        ("PUT", true),
        ("PATCH", true),
        ("DELETE", true),
        ("CONNECT", true),
        ("TRACE", false),
        ("BOGUS", false),
    ];
    for (method, authorized) in cases {
        let status = client
            .run(async move |accessor, client| {
                let method = http::Method::from_bytes(method.as_bytes())?;
                let request =
                    host_request_with(accessor, method, "https://api.example.com/", &[], b"")?;
                let response = client
                    .gated_client()
                    .call_send(accessor, request)
                    .await?
                    .map_err(|err| wasmtime::format_err!("{err:?}"))?;
                response_status(accessor, &response)
            })
            .await?;
        assert_eq!(status, 200, "{method}");
        let request = client.recorder().upstream_requests().pop().unwrap();
        assert_eq!(request.method, method);
        let expected = if authorized { vec!["Bearer secret"] } else { vec![] };
        assert_eq!(request.header("authorization"), expected, "{method}");
    }
    Ok(())
}
