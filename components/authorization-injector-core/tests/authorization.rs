//! Tests for `authorization-injector-core`, which adds an authorization header to requests sent
//! without one, from `wasi:config/store`.

use test_harness::{
    Harness, HttpErrorCode, LogEntry, UpstreamRequest, host_request_with, response_status,
};

/// Send a POST request with the headers and a body through the client, the status of the
/// response or the error.
async fn send(
    client: &mut test_harness::TestSubject,
    uri: &str,
    headers: &[(&'static str, &'static str)],
) -> wasmtime::Result<Result<u16, HttpErrorCode>> {
    send_method(client, http::Method::POST, uri, headers).await
}

/// Send a request with the method, headers and a body through the client, the status of the
/// response or the error.
async fn send_method(
    client: &mut test_harness::TestSubject,
    method: http::Method,
    uri: &str,
    headers: &[(&'static str, &'static str)],
) -> wasmtime::Result<Result<u16, HttpErrorCode>> {
    let uri = uri.to_string();
    let headers = headers.to_vec();
    client
        .run(async move |accessor, client| {
            let request =
                host_request_with(accessor, method, &uri, &headers, b"payload")?;
            Ok(
                match client.gated_client().call_send(accessor, request).await? {
                    Ok(response) => Ok(response_status(accessor, &response)?),
                    Err(err) => Err(err),
                },
            )
        })
        .await
}

/// The client, allowed to add the authorization to requests with any method.
fn harness() -> Harness {
    Harness::new("authorization-injector-core").named_config("methods", &[("*", "deferred")])
}

fn last_request(client: &test_harness::TestSubject) -> UpstreamRequest {
    client
        .recorder()
        .upstream_requests()
        .pop()
        .expect("a request was sent upstream")
}

#[tokio::test(flavor = "multi_thread")]
async fn exports_client() -> wasmtime::Result<()> {
    let client = harness().build().await?;
    client.exports().gated_client();
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn adds_authorization_for_matching_origin() -> wasmtime::Result<()> {
    let mut client = harness()
        .config("https://api.example.com", "Bearer secret")
        .build()
        .await?;
    let result = send(
        &mut client,
        "https://api.example.com/items?page=2",
        &[("x-request", "1"), ("content-type", "text/plain")],
    )
    .await?;
    assert_eq!(result.ok(), Some(200));

    let request = last_request(&client);
    assert_eq!(request.method, "POST");
    assert_eq!(request.uri, "https://api.example.com/items?page=2");
    assert_eq!(request.header("authorization"), vec!["Bearer secret"]);
    assert_eq!(request.header("x-request"), vec!["1"]);
    assert_eq!(request.header("content-type"), vec!["text/plain"]);
    assert_eq!(request.body, b"payload");
    assert_eq!(client.recorder().logs(), vec![]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn keeps_existing_authorization() -> wasmtime::Result<()> {
    let mut client = harness()
        .config("https://api.example.com", "Bearer secret")
        .build()
        .await?;
    let result = send(
        &mut client,
        "https://api.example.com/",
        &[("Authorization", "Basic mine")],
    )
    .await?;
    assert_eq!(result.ok(), Some(200));
    let request = last_request(&client);
    assert_eq!(request.header("authorization"), vec!["Basic mine"]);
    assert_eq!(request.body, b"payload");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn adds_authorization_only_to_allowed_methods() -> wasmtime::Result<()> {
    let mut client = Harness::new("authorization-injector-core")
        .named_config(
            "methods",
            &[("get", "deferred"), ("post", ""), ("trace", "denied")],
        )
        .config("https://api.example.com", "Bearer secret")
        .build()
        .await?;
    for (method, expected) in [
        ("GET", vec!["Bearer secret"]),
        ("POST", vec!["Bearer secret"]),
        ("TRACE", vec![]),
        ("trace", vec![]),
        // without a key or `*`, a method is denied
        ("PUT", vec![]),
    ] {
        let method = http::Method::from_bytes(method.as_bytes())?;
        let result = send_method(&mut client, method.clone(), "https://api.example.com/", &[]).await?;
        assert_eq!(result.ok(), Some(200), "{method}");
        let request = last_request(&client);
        assert_eq!(request.method, method.as_str());
        assert_eq!(request.header("authorization"), expected, "{method}");
        assert_eq!(request.body, b"payload");
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn without_method_config_no_authorization_is_added() -> wasmtime::Result<()> {
    let mut client = Harness::new("authorization-injector-core")
        .named_config("methods", &[])
        .config("https://api.example.com", "Bearer secret")
        .build()
        .await?;
    assert_eq!(send(&mut client, "https://api.example.com/", &[]).await?.ok(), Some(200));
    assert_eq!(last_request(&client).header("authorization"), Vec::<&str>::new());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_method_config_fails_every_request() -> wasmtime::Result<()> {
    let mut client = Harness::new("authorization-injector-core")
        .named_config("methods", &[("post", "allowed")])
        .config("https://api.example.com", "Bearer secret")
        .build()
        .await?;
    let result = send(&mut client, "https://api.example.com/", &[]).await?;
    assert!(
        matches!(
            &result,
            Err(HttpErrorCode::InternalError(Some(message)))
                if message == "invalid-config<authorization-injector-core>"
        ),
        "{result:?}"
    );
    assert_eq!(client.recorder().requests(), vec![]);
    assert_eq!(
        client.recorder().logs(),
        vec![LogEntry::critical(
            "componentized-authorization-injector",
            "Invalid config COMPONENT=authorization-injector-core CONFIG=methods KEY=post VALUE=allowed ERROR=expected one of: 'deferred', 'denied'"
        )]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn unmatched_origin_is_sent_unchanged() -> wasmtime::Result<()> {
    let mut client = harness()
        .config("https://api.example.com", "Bearer secret")
        .build()
        .await?;
    for uri in [
        "http://api.example.com/",
        "https://api.example.com:8443/",
        "https://other.example.com/",
    ] {
        assert_eq!(send(&mut client, uri, &[]).await?.ok(), Some(200));
        let request = last_request(&client);
        assert_eq!(request.header("authorization"), Vec::<&str>::new(), "{uri}");
        assert_eq!(request.body, b"payload");
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn wildcard_host_and_port() -> wasmtime::Result<()> {
    let mut client = harness()
        .config("https://*.example.com", "Bearer domain")
        .config("http://localhost:*", "Bearer local")
        .build()
        .await?;
    for (uri, expected) in [
        ("https://example.com/", vec!["Bearer domain"]),
        ("https://foo.example.com/", vec!["Bearer domain"]),
        ("https://a.b.example.com/", vec!["Bearer domain"]),
        ("https://notexample.com/", vec![]),
        ("http://localhost:8080/", vec!["Bearer local"]),
        ("http://localhost/", vec!["Bearer local"]),
    ] {
        assert_eq!(send(&mut client, uri, &[]).await?.ok(), Some(200));
        assert_eq!(last_request(&client).header("authorization"), expected, "{uri}");
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_config_fails_every_request() -> wasmtime::Result<()> {
    let mut client = harness()
        .config("api.example.com", "Bearer secret")
        .build()
        .await?;
    let result = send(&mut client, "https://api.example.com/", &[]).await?;
    assert!(
        matches!(
            &result,
            Err(HttpErrorCode::InternalError(Some(message)))
                if message == "invalid-config<authorization-injector-core>"
        ),
        "{result:?}"
    );
    assert_eq!(client.recorder().requests(), vec![]);
    assert_eq!(
        client.recorder().logs(),
        vec![LogEntry::critical(
            "componentized-authorization-injector",
            "Invalid config COMPONENT=authorization-injector-core CONFIG=credentials KEY=api.example.com ERROR=expected 'scheme://host[:port]'"
        )]
    );
    Ok(())
}
