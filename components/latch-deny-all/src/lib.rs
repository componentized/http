#![no_main]

use crate::exports::componentized::http::latch::{
    Decision, ErrorCode, Guest as Latch, HttpErrorCode, Operation,
};

struct DenyAllLatch {}

impl Latch for DenyAllLatch {
    fn authorize(_: Operation) -> Result<Option<Decision>, ErrorCode> {
        Ok(Some(Decision::Denied(HttpErrorCode::HttpRequestDenied)))
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "http-latch",
    merge_structurally_equal_types: true,
    generate_all
});

export!(DenyAllLatch);
