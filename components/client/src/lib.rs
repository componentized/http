use url::Url;

wit_bindgen::generate!({
    path: "../wit",
    world: "client",
    generate_all,
});

use exports::componentized::http::client::{
    ErrorCode, Guest, HttpResponse, Method, RedirectRequest, RequestOptions, TrailersErrorCode,
};

use wasi::http::types::{
    ErrorCode as WasiErrorCode, Fields, Method as WasiMethod, Request as WasiRequest,
    RequestOptions as WasiRequestOptions, Response as WasiResponse, Scheme, Trailers,
};
use wit_bindgen::rt::async_support::{FutureReader, StreamReader};

struct HttpClient;

/// The number of redirects followed when `request-options.max-redirects` is not set.
const DEFAULT_MAX_REDIRECTS: u8 = 10;

/// Request headers that carry credentials, not sent when a redirect leaves the origin.
const CREDENTIAL_HEADERS: [&str; 3] = ["authorization", "cookie", "proxy-authorization"];

/// Request headers that describe the body, not sent when a redirect drops the body.
const BODY_HEADERS: [&str; 5] = [
    "content-encoding",
    "content-language",
    "content-length",
    "content-location",
    "content-type",
];

impl HttpClient {
    async fn request(
        mut method: Method,
        url: String,
        mut headers: Vec<(String, String)>,
        mut body: Option<StreamReader<u8>>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        let max_redirects = options
            .as_ref()
            .and_then(|options| options.max_redirects)
            .unwrap_or(DEFAULT_MAX_REDIRECTS);
        let mut url = Url::parse(&url).map_err(|e| err(format!("Invalid URL: {e}")))?;
        let mut redirects = 0;
        loop {
            let has_body = body.is_some();
            let (response, send_result) =
                Self::send(method, &url, &headers, body.take(), options.as_ref()).await?;

            let status = response.get_status_code();
            let response_headers = read_fields(&response.get_headers());
            match next_step(
                status,
                &response_headers,
                method,
                has_body,
                redirects,
                max_redirects,
            ) {
                Step::Return => {
                    return Ok(Self::response(
                        response,
                        send_result,
                        status,
                        response_headers,
                    ));
                }
                Step::TooManyRedirects => {
                    let response = Self::response(response, send_result, status, response_headers);
                    return Err(ErrorCode::RedirectLimitExceeded((response, redirects)));
                }
                Step::RequiresBody { location } => {
                    let next = redirect_url(&url, &location)?;
                    let redirect = RedirectRequest {
                        method,
                        headers: redirect_headers(headers, &url, &next, true),
                        url: next.into(),
                        // the limit was checked, this redirect is within it
                        options: Some(redirect_options(
                            options.as_ref(),
                            max_redirects - redirects - 1,
                        )),
                    };
                    let response = Self::response(response, send_result, status, response_headers);
                    return Err(ErrorCode::RedirectRequiresBody((response, redirect)));
                }
                Step::Follow {
                    location,
                    method: next_method,
                    keep_body,
                } => {
                    let next = redirect_url(&url, &location)?;
                    headers = redirect_headers(headers, &url, &next, keep_body);
                    redirects += 1;
                    method = next_method;
                    url = next;
                }
            }
        }
    }

    /// Send a single request, without following redirects.
    async fn send(
        method: Method,
        url: &Url,
        headers: &[(String, String)],
        body: Option<StreamReader<u8>>,
        options: Option<&RequestOptions>,
    ) -> Result<(WasiResponse, FutureReader<Result<(), WasiErrorCode>>), ErrorCode> {
        let request_headers = Fields::new();
        for (name, value) in headers {
            request_headers
                .append(name, value.as_bytes())
                .map_err(|e| err(format!("Invalid request header {name:?}: {e:?}")))?;
        }

        let scheme = match url.scheme() {
            "http" => Scheme::Http,
            "https" => Scheme::Https,
            other => return Err(err(format!("Unsupported URL scheme: {other}"))),
        };
        let host = url
            .host_str()
            .ok_or_else(|| err("URL is missing a host".to_string()))?;
        let authority = match url.port() {
            Some(port) => format!("{host}:{port}"),
            None => host.to_string(),
        };
        let path_with_query = match url.query() {
            Some(q) => format!("{}?{q}", url.path()),
            None => url.path().to_string(),
        };

        let (trailers_tx, trailers_rx) = wit_future::new(|| Ok(None));
        trailers_tx.write(Ok(None));

        let wasi_options = options.map(wasi_request_options).transpose()?;
        let (request, send_result) =
            WasiRequest::new(request_headers, body, trailers_rx, wasi_options);
        request
            .set_method(&to_wasi_method(method))
            .map_err(|()| err("Failed to set request method".to_string()))?;
        request
            .set_scheme(Some(&scheme))
            .map_err(|()| err("Failed to set request scheme".to_string()))?;
        request
            .set_authority(Some(&authority))
            .map_err(|()| err("Failed to set request authority".to_string()))?;
        request
            .set_path_with_query(Some(&path_with_query))
            .map_err(|()| err("Failed to set request path".to_string()))?;

        let response = wasi::http::client::send(request)
            .await
            .map_err(|e| err(format!("HTTP request failed: {e:?}")))?;
        Ok((response, send_result))
    }

    fn response(
        response: WasiResponse,
        send_result: FutureReader<Result<(), WasiErrorCode>>,
        status: u16,
        headers: Vec<(String, String)>,
    ) -> HttpResponse {
        let (body_stream, wasi_trailers) = WasiResponse::consume_body(response, send_result);
        HttpResponse {
            status,
            headers,
            body: body_stream,
            trailers: map_trailers(wasi_trailers),
        }
    }
}

/// What to do with a response.
#[derive(Debug)]
enum Step {
    /// Return the response to the caller.
    Return,
    /// Send a request to the location of the redirect.
    Follow {
        location: String,
        method: Method,
        keep_body: bool,
    },
    /// The redirect sends the body again, which the caller has to do.
    RequiresBody { location: String },
    /// The response is a redirect past the limit.
    TooManyRedirects,
}

/// What to do with a response, after following `redirects` redirects of at most `max_redirects`.
///
/// A redirect is followed when its location is known, unless it sends the request body again.
/// The body was streamed to the first request, it can't be sent again.
fn next_step(
    status: u16,
    headers: &[(String, String)],
    method: Method,
    has_body: bool,
    redirects: u8,
    max_redirects: u8,
) -> Step {
    if max_redirects == 0 {
        return Step::Return;
    }
    let Some(location) = redirect_location(status, headers) else {
        return Step::Return;
    };
    if redirects >= max_redirects {
        return Step::TooManyRedirects;
    }
    let (method, keep_body) = redirect_method(status, method);
    if has_body && keep_body {
        return Step::RequiresBody { location };
    }
    Step::Follow {
        location,
        method,
        keep_body,
    }
}

/// The location to follow for a redirect response, `None` if the response is not a redirect.
fn redirect_location(status: u16, headers: &[(String, String)]) -> Option<String> {
    if !matches!(status, 301 | 302 | 303 | 307 | 308) {
        return None;
    }
    headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("location"))
        .map(|(_, value)| value.clone())
}

/// The method for the request following a redirect, and whether the body is sent again.
///
/// 303 switches to GET, except for HEAD, as does 301 and 302 for POST, as browsers do. Otherwise
/// the method and body are kept, see RFC 9110 section 15.4.
fn redirect_method(status: u16, method: Method) -> (Method, bool) {
    match (status, method) {
        (303, Method::Head) => (Method::Head, false),
        (303, _) | (301 | 302, Method::Post) => (Method::Get, false),
        _ => (method, true),
    }
}

/// The options for the request following a redirect, the original options with the redirects
/// remaining.
fn redirect_options(options: Option<&RequestOptions>, max_redirects: u8) -> RequestOptions {
    RequestOptions {
        connect_timeout_ms: options.and_then(|options| options.connect_timeout_ms),
        first_byte_timeout_ms: options.and_then(|options| options.first_byte_timeout_ms),
        between_bytes_timeout_ms: options.and_then(|options| options.between_bytes_timeout_ms),
        max_redirects: Some(max_redirects),
    }
}

/// The URL a redirect location refers to, relative to the URL of the request.
fn redirect_url(url: &Url, location: &str) -> Result<Url, ErrorCode> {
    url.join(location)
        .map_err(|e| err(format!("Invalid redirect location {location:?}: {e}")))
}

/// The request headers for the request following a redirect from `from` to `to`.
///
/// Credentials are not sent to another origin, and headers describing the body are not sent
/// without the body.
fn redirect_headers(
    mut headers: Vec<(String, String)>,
    from: &Url,
    to: &Url,
    keep_body: bool,
) -> Vec<(String, String)> {
    if from.origin() != to.origin() {
        headers.retain(|(name, _)| !is_one_of(name, &CREDENTIAL_HEADERS));
    }
    if !keep_body {
        headers.retain(|(name, _)| !is_one_of(name, &BODY_HEADERS));
    }
    headers
}

fn is_one_of(name: &str, names: &[&str]) -> bool {
    names.iter().any(|n| name.eq_ignore_ascii_case(n))
}

fn err(message: String) -> ErrorCode {
    ErrorCode::Other(Some(message))
}

fn read_fields(fields: &Fields) -> Vec<(String, String)> {
    fields
        .copy_all()
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().map(|b| b as char).collect()))
        .collect()
}

fn map_trailers(
    wasi: FutureReader<Result<Option<Trailers>, WasiErrorCode>>,
) -> FutureReader<Result<Vec<(String, String)>, TrailersErrorCode>> {
    let (tx, rx) = wit_future::new(|| Ok(Vec::new()));
    wit_bindgen::rt::async_support::spawn_local(async move {
        let resolved = match wasi.await {
            Ok(Some(t)) => Ok(read_fields(&t)),
            Ok(None) => Ok(Vec::new()),
            Err(e) => Err(TrailersErrorCode::Other(Some(format!(
                "wasi:http error: {e:?}"
            )))),
        };
        tx.write(resolved);
    });
    rx
}

fn wasi_request_options(opts: &RequestOptions) -> Result<WasiRequestOptions, ErrorCode> {
    let r = WasiRequestOptions::new();
    if let Some(ms) = opts.connect_timeout_ms {
        r.set_connect_timeout(Some(ms_to_ns(ms)))
            .map_err(|e| err(format!("connect-timeout: {e:?}")))?;
    }
    if let Some(ms) = opts.first_byte_timeout_ms {
        r.set_first_byte_timeout(Some(ms_to_ns(ms)))
            .map_err(|e| err(format!("first-byte-timeout: {e:?}")))?;
    }
    if let Some(ms) = opts.between_bytes_timeout_ms {
        r.set_between_bytes_timeout(Some(ms_to_ns(ms)))
            .map_err(|e| err(format!("between-bytes-timeout: {e:?}")))?;
    }
    Ok(r)
}

fn ms_to_ns(ms: u32) -> u64 {
    u64::from(ms) * 1_000_000
}

fn to_wasi_method(method: Method) -> WasiMethod {
    match method {
        Method::Get => WasiMethod::Get,
        Method::Post => WasiMethod::Post,
        Method::Put => WasiMethod::Put,
        Method::Delete => WasiMethod::Delete,
        Method::Patch => WasiMethod::Patch,
        Method::Head => WasiMethod::Head,
        Method::Options => WasiMethod::Options,
        Method::Trace => WasiMethod::Trace,
        Method::Query => WasiMethod::Other("QUERY".to_string()),
    }
}

impl Guest for HttpClient {
    async fn request(
        method: Method,
        url: String,
        headers: Vec<(String, String)>,
        body: Option<StreamReader<u8>>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        Self::request(method, url, headers, body, options).await
    }

    async fn get(
        url: String,
        headers: Vec<(String, String)>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        Self::request(Method::Get, url, headers, None, options).await
    }

    async fn post(
        url: String,
        headers: Vec<(String, String)>,
        body: StreamReader<u8>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        Self::request(Method::Post, url, headers, Some(body), options).await
    }

    async fn put(
        url: String,
        headers: Vec<(String, String)>,
        body: StreamReader<u8>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        Self::request(Method::Put, url, headers, Some(body), options).await
    }

    async fn delete(
        url: String,
        headers: Vec<(String, String)>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        Self::request(Method::Delete, url, headers, None, options).await
    }

    async fn patch(
        url: String,
        headers: Vec<(String, String)>,
        body: StreamReader<u8>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        Self::request(Method::Patch, url, headers, Some(body), options).await
    }

    async fn head(
        url: String,
        headers: Vec<(String, String)>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        Self::request(Method::Head, url, headers, None, options).await
    }

    async fn options(
        url: String,
        headers: Vec<(String, String)>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        Self::request(Method::Options, url, headers, None, options).await
    }

    async fn trace(
        url: String,
        headers: Vec<(String, String)>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        Self::request(Method::Trace, url, headers, None, options).await
    }

    async fn query(
        url: String,
        headers: Vec<(String, String)>,
        body: StreamReader<u8>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        Self::request(Method::Query, url, headers, Some(body), options).await
    }
}

export!(HttpClient);

#[cfg(test)]
mod tests {
    use super::*;

    fn location(value: &str) -> Vec<(String, String)> {
        vec![("Location".to_string(), value.to_string())]
    }

    fn url(value: &str) -> Url {
        Url::parse(value).unwrap()
    }

    /// Compare by debug output, the generated `Method` does not implement `PartialEq`.
    fn assert_same(actual: impl std::fmt::Debug, expected: impl std::fmt::Debug, context: &str) {
        assert_eq!(format!("{actual:?}"), format!("{expected:?}"), "{context}");
    }

    fn header(name: &str) -> (String, String) {
        (name.to_string(), "value".to_string())
    }

    #[test]
    fn follows_redirects() {
        for status in [301, 302, 303, 307, 308] {
            assert!(
                matches!(
                    next_step(status, &location("/next"), Method::Get, false, 0, 10),
                    Step::Follow { ref location, .. } if location == "/next"
                ),
                "{status}"
            );
        }
    }

    #[test]
    fn returns_other_responses() {
        for status in [200, 204, 300, 304, 404, 500] {
            assert_same(
                next_step(status, &location("/next"), Method::Get, false, 0, 10),
                Step::Return,
                &format!("{status}"),
            );
        }
    }

    #[test]
    fn returns_redirect_without_location() {
        assert_same(
            next_step(302, &[], Method::Get, false, 0, 10),
            Step::Return,
            "",
        );
    }

    #[test]
    fn zero_max_redirects_returns_redirect() {
        assert_same(
            next_step(302, &location("/next"), Method::Get, false, 0, 0),
            Step::Return,
            "",
        );
    }

    #[test]
    fn exceeding_max_redirects_is_an_error() {
        assert!(matches!(
            next_step(302, &location("/next"), Method::Get, false, 2, 3),
            Step::Follow { .. }
        ));
        assert_same(
            next_step(302, &location("/next"), Method::Get, false, 3, 3),
            Step::TooManyRedirects,
            "",
        );
    }

    #[test]
    fn redirect_that_resends_the_body_requires_the_caller() {
        for status in [307, 308] {
            assert_same(
                next_step(status, &location("/next"), Method::Post, true, 0, 10),
                Step::RequiresBody {
                    location: "/next".to_string(),
                },
                &format!("{status}"),
            );
        }
        // without a body there is nothing to resend
        assert_same(
            next_step(307, &location("/next"), Method::Post, false, 0, 10),
            Step::Follow {
                location: "/next".to_string(),
                method: Method::Post,
                keep_body: true,
            },
            "",
        );
    }

    #[test]
    fn moved_redirect_for_a_method_other_than_post_requires_the_body() {
        for status in [301, 302] {
            for method in [Method::Put, Method::Patch, Method::Delete, Method::Query] {
                assert_same(
                    next_step(status, &location("/next"), method, true, 0, 10),
                    Step::RequiresBody {
                        location: "/next".to_string(),
                    },
                    &format!("{status} {method:?}"),
                );
            }
            // POST switches to GET without the body
            assert_same(
                next_step(status, &location("/next"), Method::Post, true, 0, 10),
                Step::Follow {
                    location: "/next".to_string(),
                    method: Method::Get,
                    keep_body: false,
                },
                &format!("{status}"),
            );
        }
    }

    #[test]
    fn redirect_limit_is_checked_before_the_body() {
        assert_same(
            next_step(307, &location("/next"), Method::Post, true, 3, 3),
            Step::TooManyRedirects,
            "",
        );
    }

    #[test]
    fn redirect_that_drops_the_body_is_followed() {
        assert_same(
            next_step(303, &location("/next"), Method::Put, true, 0, 10),
            Step::Follow {
                location: "/next".to_string(),
                method: Method::Get,
                keep_body: false,
            },
            "",
        );
    }

    #[test]
    fn redirect_methods() {
        use Method::*;
        let cases = [
            (301, Post, Get, false),
            (302, Post, Get, false),
            (303, Post, Get, false),
            (303, Put, Get, false),
            (303, Get, Get, false),
            (303, Head, Head, false),
            (301, Put, Put, true),
            (302, Delete, Delete, true),
            (307, Post, Post, true),
            (308, Put, Put, true),
        ];
        for (status, method, expected, keep_body) in cases {
            assert_same(
                redirect_method(status, method),
                (expected, keep_body),
                &format!("{status} {method:?}"),
            );
        }
    }

    #[test]
    fn credentials_kept_for_the_same_origin() {
        let headers = vec![header("Authorization"), header("Cookie"), header("accept")];
        assert_eq!(
            redirect_headers(
                headers.clone(),
                &url("https://example.com/a"),
                &url("https://example.com/b"),
                true
            ),
            headers
        );
    }

    #[test]
    fn credentials_dropped_for_another_origin() {
        let headers = vec![
            header("Authorization"),
            header("cookie"),
            header("Proxy-Authorization"),
            header("accept"),
        ];
        for to in [
            "https://example.org/",
            "https://other.example.com/",
            "http://example.com/",
            "https://example.com:8443/",
        ] {
            assert_eq!(
                redirect_headers(
                    headers.clone(),
                    &url("https://example.com/"),
                    &url(to),
                    true
                ),
                vec![header("accept")],
                "{to}"
            );
        }
    }

    #[test]
    fn body_headers_dropped_with_the_body() {
        let headers = vec![
            header("Content-Type"),
            header("content-length"),
            header("Content-Encoding"),
            header("Content-Language"),
            header("Content-Location"),
            header("accept"),
        ];
        let same = url("https://example.com/");
        assert_eq!(
            redirect_headers(headers.clone(), &same, &same, false),
            vec![header("accept")]
        );
        assert_eq!(
            redirect_headers(headers.clone(), &same, &same, true),
            headers
        );
    }

    #[test]
    fn redirect_options_copy_the_original_options() {
        let original = RequestOptions {
            connect_timeout_ms: Some(1),
            first_byte_timeout_ms: Some(2),
            between_bytes_timeout_ms: None,
            max_redirects: Some(5),
        };
        assert_same(
            redirect_options(Some(&original), 3),
            RequestOptions {
                connect_timeout_ms: Some(1),
                first_byte_timeout_ms: Some(2),
                between_bytes_timeout_ms: None,
                max_redirects: Some(3),
            },
            "",
        );
        assert_same(
            redirect_options(None, 9),
            RequestOptions {
                connect_timeout_ms: None,
                first_byte_timeout_ms: None,
                between_bytes_timeout_ms: None,
                max_redirects: Some(9),
            },
            "",
        );
    }
}
