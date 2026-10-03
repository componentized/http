//! Tests for the redirects followed by `client`, against a scripted upstream.

use test_harness::{
    Harness, HttpClientErrorCode, HttpClientMethod, HttpClientRedirectRequest,
    HttpClientRequestOptions, UpstreamRequest, UpstreamResponse, stream,
};

/// Answer each path with the response, and every other path with the default response.
fn routes(
    routes: Vec<(&'static str, UpstreamResponse)>,
) -> impl FnMut(&UpstreamRequest) -> UpstreamResponse + Send + 'static {
    move |request| {
        routes
            .iter()
            .find(|(path, _)| request.path() == *path)
            .map(|(_, response)| response.clone())
            .unwrap_or_default()
    }
}

fn options(max_redirects: Option<u8>) -> Option<HttpClientRequestOptions> {
    Some(HttpClientRequestOptions {
        connect_timeout_ms: Some(1_000),
        first_byte_timeout_ms: None,
        between_bytes_timeout_ms: None,
        max_redirects,
    })
}

fn header(name: &str, value: &str) -> (String, String) {
    (name.to_string(), value.to_string())
}

/// The method and path of each request sent upstream.
fn sent(client: &test_harness::TestSubject) -> Vec<(String, String)> {
    client
        .recorder()
        .upstream_requests()
        .iter()
        .map(|request| (request.method.clone(), request.path().to_string()))
        .collect()
}

fn get(path: &str) -> (String, String) {
    ("GET".to_string(), path.to_string())
}

#[tokio::test(flavor = "multi_thread")]
async fn follows_redirects() -> wasmtime::Result<()> {
    let mut client = Harness::new("client")
        .upstream(routes(vec![
            ("/a", UpstreamResponse::redirect(301, "/b")),
            (
                "/b",
                UpstreamResponse::redirect(302, "https://example.com/c"),
            ),
            ("/c", UpstreamResponse::redirect(307, "d")),
            ("/d", UpstreamResponse::redirect(308, "/e?f=g")),
        ]))
        .build()
        .await?;
    let status = client
        .run(async |accessor, client| {
            let response = client
                .componentized_http_client()
                .call_get(accessor, "https://example.com/a".to_string(), vec![], None)
                .await?
                .expect("get");
            Ok(response.status)
        })
        .await?;
    assert_eq!(status, 200);
    assert_eq!(
        sent(&client),
        vec![get("/a"), get("/b"), get("/c"), get("/d"), get("/e?f=g")]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn zero_max_redirects_returns_the_redirect() -> wasmtime::Result<()> {
    let mut client = Harness::new("client")
        .upstream(routes(vec![("/a", UpstreamResponse::redirect(302, "/b"))]))
        .build()
        .await?;
    let (status, headers) = client
        .run(async |accessor, client| {
            let response = client
                .componentized_http_client()
                .call_get(
                    accessor,
                    "https://example.com/a".to_string(),
                    vec![],
                    options(Some(0)),
                )
                .await?
                .expect("get");
            Ok((response.status, response.headers))
        })
        .await?;
    assert_eq!(status, 302);
    assert!(headers.contains(&header("location", "/b")), "{headers:?}");
    assert_eq!(sent(&client), vec![get("/a")]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn exceeding_max_redirects_is_an_error() -> wasmtime::Result<()> {
    let mut client = Harness::new("client")
        .upstream(|_| UpstreamResponse::redirect(302, "/loop"))
        .build()
        .await?;
    let error = client
        .run(async |accessor, client| {
            let result = client
                .componentized_http_client()
                .call_get(
                    accessor,
                    "https://example.com/start".to_string(),
                    vec![],
                    options(Some(2)),
                )
                .await?;
            Ok(match result {
                Err(HttpClientErrorCode::RedirectLimitExceeded((response, redirects))) => {
                    Some((response.status, redirects))
                }
                _ => None,
            })
        })
        .await?;
    // the redirect response past the limit, after following 2 redirects
    assert_eq!(error, Some((302, 2)));
    assert_eq!(
        sent(&client),
        vec![get("/start"), get("/loop"), get("/loop")]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn see_other_switches_to_get_without_the_body() -> wasmtime::Result<()> {
    let mut client = Harness::new("client")
        .upstream(routes(vec![("/a", UpstreamResponse::redirect(303, "/b"))]))
        .build()
        .await?;
    let status = client
        .run(async |accessor, client| {
            let body = stream(accessor, b"payload".to_vec())?;
            let response = client
                .componentized_http_client()
                .call_post(
                    accessor,
                    "https://example.com/a".to_string(),
                    vec![
                        header("content-type", "text/plain"),
                        header("accept", "text/plain"),
                    ],
                    body,
                    None,
                )
                .await?
                .expect("post");
            Ok(response.status)
        })
        .await?;
    assert_eq!(status, 200);
    let requests = client.recorder().upstream_requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].body, b"payload");
    assert_eq!(requests[1].method, "GET");
    assert_eq!(requests[1].path(), "/b");
    assert_eq!(requests[1].body, b"");
    // headers describing the body are dropped with it
    assert_eq!(requests[1].header("content-type"), Vec::<&str>::new());
    assert_eq!(requests[1].header("accept"), vec!["text/plain"]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn credentials_are_not_sent_to_another_origin() -> wasmtime::Result<()> {
    let mut client = Harness::new("client")
        .upstream(routes(vec![
            ("/a", UpstreamResponse::redirect(302, "/b")),
            (
                "/b",
                UpstreamResponse::redirect(302, "https://example.org/c"),
            ),
        ]))
        .build()
        .await?;
    client
        .run(async |accessor, client| {
            client
                .componentized_http_client()
                .call_get(
                    accessor,
                    "https://example.com/a".to_string(),
                    vec![
                        header("authorization", "Bearer secret"),
                        header("accept", "text/plain"),
                    ],
                    None,
                )
                .await?
                .expect("get");
            Ok(())
        })
        .await?;
    let requests = client.recorder().upstream_requests();
    assert_eq!(requests.len(), 3);
    // kept for the same origin
    assert_eq!(requests[1].header("authorization"), vec!["Bearer secret"]);
    // dropped for another origin
    assert_eq!(requests[2].uri, "https://example.org/c");
    assert_eq!(requests[2].header("authorization"), Vec::<&str>::new());
    assert_eq!(requests[2].header("accept"), vec!["text/plain"]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn redirect_that_resends_the_body_is_returned_to_the_caller() -> wasmtime::Result<()> {
    let mut client = Harness::new("client")
        .upstream(routes(vec![(
            "/start",
            UpstreamResponse::redirect(307, "https://example.org/b"),
        )]))
        .build()
        .await?;
    let (status, redirect) = client
        .run(async |accessor, client| {
            let body = stream(accessor, b"payload".to_vec())?;
            let result = client
                .componentized_http_client()
                .call_request(
                    accessor,
                    HttpClientMethod::Put,
                    "https://example.com/start".to_string(),
                    vec![
                        header("authorization", "Bearer secret"),
                        header("content-type", "text/plain"),
                    ],
                    Some(body),
                    options(Some(5)),
                )
                .await?;
            Ok(match result {
                Err(HttpClientErrorCode::RedirectRequiresBody((response, redirect))) => {
                    (response.status, redirect)
                }
                Ok(response) => panic!("expected redirect-requires-body, got {}", response.status),
                Err(err) => panic!("expected redirect-requires-body, got {err:?}"),
            })
        })
        .await?;
    assert_eq!(status, 307);
    // returned without following it, the body was already sent
    assert_eq!(
        sent(&client),
        vec![("PUT".to_string(), "/start".to_string())]
    );
    assert!(
        matches!(redirect.method, HttpClientMethod::Put),
        "{:?}",
        redirect.method
    );
    assert_eq!(redirect.url, "https://example.org/b");
    // the credentials are dropped for another origin, the body headers are kept for the body
    assert_eq!(redirect.headers, vec![header("content-type", "text/plain")]);
    // the original options, with the redirects remaining after this one of 5
    let options = redirect.options.expect("options");
    assert_eq!(options.connect_timeout_ms, Some(1_000));
    assert_eq!(options.max_redirects, Some(4));
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn caller_resends_the_body_to_follow_the_redirect() -> wasmtime::Result<()> {
    let mut client = Harness::new("client")
        .upstream(routes(vec![("/a", UpstreamResponse::redirect(308, "/b"))]))
        .build()
        .await?;
    let status = client
        .run(async |accessor, client| {
            let http = client.componentized_http_client();
            let body = stream(accessor, b"payload".to_vec())?;
            let result = http
                .call_post(
                    accessor,
                    "https://example.com/a".to_string(),
                    vec![header("content-type", "text/plain")],
                    body,
                    None,
                )
                .await?;
            let HttpClientRedirectRequest {
                method,
                url,
                headers,
                options,
            } = match result {
                Err(HttpClientErrorCode::RedirectRequiresBody((_, redirect))) => redirect,
                Ok(response) => panic!("expected redirect-requires-body, got {}", response.status),
                Err(err) => panic!("expected redirect-requires-body, got {err:?}"),
            };
            // send the body again, as given
            let body = stream(accessor, b"payload".to_vec())?;
            let response = http
                .call_request(accessor, method, url, headers, Some(body), options)
                .await?
                .expect("request");
            Ok(response.status)
        })
        .await?;
    assert_eq!(status, 200);
    let requests = client.recorder().upstream_requests();
    assert_eq!(requests.len(), 2);
    for request in &requests {
        assert_eq!(request.method, "POST");
        assert_eq!(request.body, b"payload");
        assert_eq!(request.header("content-type"), vec!["text/plain"]);
    }
    assert_eq!(requests[1].path(), "/b");
    Ok(())
}
