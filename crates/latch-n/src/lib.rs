#![no_main]

use crate::bindings::exports::componentized::http::latch::{Decision, ErrorCode, Operation};

pub fn authorize(
    operation: Operation,
    authorizers: Vec<fn(&Operation<'_>) -> Result<Decision, ErrorCode>>,
) -> Result<Decision, ErrorCode> {
    for authorize in authorizers {
        match authorize(&operation)? {
            Decision::Abstained => {}
            Decision::Denied(error_code) => return Ok(Decision::Denied(error_code)),
        }
    }
    Ok(Decision::Abstained)
}

pub mod bindings {
    wit_bindgen::generate!({
        path: "../../components/wit",
        world: "http-latch-n",
        pub_export_macro: true,
        merge_structurally_equal_types: true,
        generate_all
    });
}

#[macro_export]
macro_rules! export {
    ($($t:tt)*) => {
        $crate::bindings::export!($($t)*);
    };
}
