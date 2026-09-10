#![no_main]

use crate::exports::componentized::http::latch::{Decision, ErrorCode, Guest as Latch, Operation};

struct GrantAllLatch {}

impl Latch for GrantAllLatch {
    fn authorize(_: Operation) -> Result<Decision, ErrorCode> {
        Ok(Decision::Abstained)
    }
}

wit_bindgen::generate!({
    path: "../wit",
    world: "http-latch",
    merge_structurally_equal_types: true,
    generate_all
});

export!(GrantAllLatch);
