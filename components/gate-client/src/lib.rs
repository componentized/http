#![no_main]

use std::fmt::Display;

use crate::{
    componentized::http::latch::{
        self, authorize, ClientOperation, Decision::Denied, Operation, SendArgs,
    },
    exports::wasi::http::client::{ErrorCode, Guest, Request, Response},
    wasi::{
        http::{client, types},
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

struct GatedHttpClient {}

impl Guest for GatedHttpClient {
    #[doc = "/ This function may be used to either send an outgoing request over the"]
    #[doc = "/ network or to forward it to another component."]
    #[allow(async_fn_in_trait)]
    async fn send(request: Request) -> Result<Response, ErrorCode> {
        match authorize(&Operation::Client(ClientOperation::Send(SendArgs {
            request: &request,
        })))? {
            Some(Denied(reason)) => {
                warn!(
                    "Denied REASON={reason} OPERATION=wasi:http/client#send METHOD={} PATH={}",
                    request.get_method(),
                    request.get_path_with_query().unwrap_or("/".to_string())
                );
                Err(reason)
            }
            _ => client::send(request).await,
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
    world: "gate-client",
    merge_structurally_equal_types: true,
    generate_all
});

export!(GatedHttpClient);
