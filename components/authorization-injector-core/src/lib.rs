mod rules;

use crate::{
    exports::wasi::http::client::{Guest, Request, Response},
    rules::{MethodRules, Origin, Rules},
    wasi::{
        config::store,
        http::{
            client,
            types::{ErrorCode, Scheme},
        },
        logging::logging::{Level, log},
    },
};

const COMPONENT_NAME: &str = "authorization-injector-core";

const LOG_CONTEXT: &str = "componentized-authorization-injector";

const AUTHORIZATION: &str = "authorization";

struct AuthorizationInjector {}

impl Guest for AuthorizationInjector {
    #[doc = "/ This function may be used to either send an outgoing request over the"]
    #[doc = "/ network or to forward it to another component."]
    #[allow(async_fn_in_trait)]
    async fn send(request: Request) -> Result<Response, ErrorCode> {
        match authorization(&request)? {
            Some(authorization) => client::send(with_authorization(request, &authorization)?).await,
            None => client::send(request).await,
        }
    }
}

/// The authorization to add to the request, `None` when the request already has an authorization
/// header, its method is not allowed, or none is configured for its origin.
fn authorization(request: &Request) -> Result<Option<String>, ErrorCode> {
    if request.get_headers().has(AUTHORIZATION) {
        return Ok(None);
    }
    let methods = load_config(
        "methods",
        methods::get_all().map_err(|err| match err {
            methods::Error::Upstream(message) => format!("ERROR=upstream: {message}"),
            methods::Error::Io(message) => format!("ERROR=io: {message}"),
        }),
        MethodRules::parse,
    )?;
    let method = http_utils::format_http_method!(wasi::http::types::Method, request.get_method());
    if !methods.allows(&method) {
        return Ok(None);
    }
    let rules = load_config(
        "credentials",
        store::get_all().map_err(|err| match err {
            store::Error::Upstream(message) => format!("ERROR=upstream: {message}"),
            store::Error::Io(message) => format!("ERROR=io: {message}"),
        }),
        Rules::parse,
    )?;
    let (Some(scheme), Some(authority)) = (request.get_scheme(), request.get_authority()) else {
        return Ok(None);
    };
    let scheme = http_utils::format_http_scheme!(Scheme, &scheme);
    Ok(Origin::new(&scheme, &authority)
        .and_then(|origin| rules.authorization(&origin).map(str::to_string)))
}

/// Parse the entries of a config, `credentials` for `wasi:config/store` or `methods` for the
/// `methods` import. An invalid config fails every request, so a typo is noticed rather than
/// ignored.
fn load_config<T>(
    config: &str,
    entries: Result<Vec<(String, String)>, String>,
    parse: impl FnOnce(Vec<(String, String)>) -> Result<T, String>,
) -> Result<T, ErrorCode> {
    entries.and_then(parse).map_err(|message| {
        log(
            Level::Critical,
            LOG_CONTEXT,
            &format!("Invalid config COMPONENT={COMPONENT_NAME} CONFIG={config} {message}"),
        );
        ErrorCode::InternalError(Some(format!("invalid-config<{COMPONENT_NAME}>")))
    })
}

/// The request with the authorization header added.
///
/// The headers of a request are immutable, so a new request is made with a copy of the headers,
/// the same method, target and options, and the body and trailers of the original request. The
/// result of transmitting the new request is forwarded to the original request.
fn with_authorization(request: Request, authorization: &str) -> Result<Request, ErrorCode> {
    let headers = request.get_headers().clone();
    headers
        .set(AUTHORIZATION, &[authorization.as_bytes().to_vec()])
        .map_err(|err| internal_error(format!("failed to set the authorization header: {err:?}")))?;
    // a child of the request, dropped before the request is consumed
    let options = request.get_options().map(|options| options.clone());
    let method = request.get_method();
    let scheme = request.get_scheme();
    let authority = request.get_authority();
    let path_with_query = request.get_path_with_query();

    let (res_tx, res_rx) = wit_future::new(|| Ok(()));
    let (contents, trailers) = Request::consume_body(request, res_rx);
    let (request, transmitted) = Request::new(headers, Some(contents), trailers, options);
    wit_bindgen::spawn_local(async move {
        let _ = res_tx.write(transmitted.await).await;
    });

    let invalid = |part: &str| internal_error(format!("failed to copy the request {part}"));
    request.set_method(&method).map_err(|()| invalid("method"))?;
    request
        .set_scheme(scheme.as_ref())
        .map_err(|()| invalid("scheme"))?;
    request
        .set_authority(authority.as_deref())
        .map_err(|()| invalid("authority"))?;
    request
        .set_path_with_query(path_with_query.as_deref())
        .map_err(|()| invalid("path with query"))?;
    Ok(request)
}

fn internal_error(message: String) -> ErrorCode {
    log(Level::Error, LOG_CONTEXT, &message);
    ErrorCode::InternalError(Some(message))
}

wit_bindgen::generate!({
    path: "../wit",
    world: "authorization-injector",
    generate_all
});

export!(AuthorizationInjector);
