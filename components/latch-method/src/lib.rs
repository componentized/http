#![no_main]

use crate::{
    exports::componentized::http::latch::{
        ClientOperation, Decision, ErrorCode, Guest as Latch, HandlerOperation, HttpErrorCode,
        Operation,
    },
    wasi::{config::store as config, http::types::Method},
};

const ABSTAINED: &str = "abstained";
const DENIED: &str = "denied";
const WILDCARD: &str = "*";

struct MethodLatch {}

impl MethodLatch {
    fn authorize_method(method: Method) -> Result<Decision, ErrorCode> {
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

        match config::get(method)? {
            Some(method_value) => Self::parse_decision(method_value),
            None => match config::get(WILDCARD)? {
                Some(default_value) => Self::parse_decision(default_value),
                None => Ok(Decision::Abstained),
            },
        }
    }

    fn parse_decision(value: String) -> Result<Decision, ErrorCode> {
        match value.as_str() {
            "" | ABSTAINED => Ok(Decision::Abstained),
            DENIED => Ok(Decision::Denied(HttpErrorCode::HttpRequestMethodInvalid)),
            val => Err(ErrorCode::Other(Some(format!(
                "unknown decision value '{val}', expected one of: '{ABSTAINED}', '{DENIED}'"
            )))),
        }
    }
}

impl Latch for MethodLatch {
    fn authorize(op: Operation) -> Result<Decision, ErrorCode> {
        match op {
            Operation::Client(client_operation) => match client_operation {
                ClientOperation::Send(args) => Self::authorize_method(args.request.get_method()),
            },
            Operation::Handler(handler_operation) => match handler_operation {
                HandlerOperation::Handle(args) => Self::authorize_method(args.request.get_method()),
            },
        }
    }
}

impl From<config::Error> for ErrorCode {
    fn from(value: config::Error) -> Self {
        match value {
            config::Error::Upstream(error) => Self::Other(Some(error)),
            config::Error::Io(error) => Self::Other(Some(error)),
        }
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "http-latch",
    merge_structurally_equal_types: true,
    generate_all
});

export!(MethodLatch);
