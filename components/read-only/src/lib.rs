#![no_std]

use crate::{
    exports::wasi::http::client::{ErrorCode, Guest, Request, Response},
    wasi::http::{client, types::Method},
};

struct ReadOnlyClient;

impl ReadOnlyClient {
    fn allowed_method(method: Method) -> bool {
        let method = match method {
            Method::Get => "get",
            Method::Head => "head",
            Method::Post => "post",
            Method::Put => "put",
            Method::Delete => "delete",
            Method::Connect => "connect",
            Method::Options => "options",
            Method::Trace => "trace",
            Method::Patch => "patch",
            Method::Other(method) => &method.to_lowercase(),
        };
        match method {
            "get" | "head" | "query" | "options" => true,
            _ => false,
        }
    }
}

impl Guest for ReadOnlyClient {
    #[doc = "/ This function may be used to either send an outgoing request over the"]
    #[doc = "/ network or to forward it to another component."]
    #[allow(async_fn_in_trait)]
    async fn send(request: Request) -> Result<Response, ErrorCode> {
        match Self::allowed_method(request.get_method()) {
            true => client::send(request).await,
            false => Err(ErrorCode::HttpRequestMethodInvalid),
        }
    }
}

export!(ReadOnlyClient);

wit_bindgen::generate!({
    path: "../wit",
    world: "client",
    generate_all,
});
