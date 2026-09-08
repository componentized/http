#![no_main]

use std::fmt::Display;

use crate::{
    componentized::http::latch::{
        self, authorize, Decision::Denied, HandleArgs, HandlerOperation, Operation,
    },
    exports::wasi::http::handler::{ErrorCode, Guest, Request, Response},
    wasi::{
        http::{handler, types},
        logging::logging::{log, Level},
    },
};

macro_rules! warn {
    ($dst:expr, $($arg:tt)*) => {
        log(Level::Warn, "componentized-gate", &format!($dst, $($arg)*));
    };
    ($dst:expr) => {
        log(Level::Warn, "componentized-gate", &format!($dst));
    };
}

struct GatedHttpHandler {}

impl Guest for GatedHttpHandler {
    #[doc = "/ This function may be called with either an incoming request read from the"]
    #[doc = "/ network or a request synthesized or forwarded by another component."]
    #[allow(async_fn_in_trait)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        match authorize(&Operation::Handler(HandlerOperation::Handle(HandleArgs {
            request: &request,
        })))? {
            Some(Denied(reason)) => {
                warn!(
                    "Denied REASON={reason} OPERATION=wasi:http/handler#handle METHOD={} PATH={}",
                    request.get_method(),
                    request.get_path_with_query().unwrap_or("/".to_string())
                );
                Err(reason)
            }
            _ => handler::handle(request).await,
        }
    }
}

impl Display for types::Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let method = self.get_method();
        let url = self.get_path_with_query().unwrap_or("/".to_string());
        f.write_fmt(format_args!("{method} {url}"))
    }
}

impl Display for types::Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let method = match self {
            types::Method::Get => "GET",
            types::Method::Head => "HEAD",
            types::Method::Post => "POST",
            types::Method::Put => "PUT",
            types::Method::Delete => "DELETE",
            types::Method::Connect => "CONNECT",
            types::Method::Options => "OPTIONS",
            types::Method::Trace => "TRACE",
            types::Method::Patch => "PATCH",
            types::Method::Other(method) => &method.to_uppercase(),
        };
        f.write_str(method)
    }
}

impl From<latch::ErrorCode> for ErrorCode {
    fn from(value: latch::ErrorCode) -> Self {
        match value {
            latch::ErrorCode::Http(error_code) => error_code,
            latch::ErrorCode::Other(error_code) => Self::InternalError(error_code),
        }
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "gate-handler",
    merge_structurally_equal_types: true,
    generate_all
});

export!(GatedHttpHandler);
