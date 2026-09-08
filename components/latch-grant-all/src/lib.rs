#![no_main]

use crate::exports::componentized::http::latch::{Decision, ErrorCode, Guest as Latch, Operation};

struct GrantAllLatch {}

impl Latch for GrantAllLatch {
    fn authorize(_: Operation) -> Result<Option<Decision>, ErrorCode> {
        Ok(Some(Decision::Granted))
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "http-latch",
    merge_structurally_equal_types: true,
    generate_all
});

export!(GrantAllLatch);
