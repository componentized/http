#![no_main]

use crate::bindings::exports::componentized::http::latch::{Decision, ErrorCode, Operation};

pub fn authorize(
    operation: Operation,
    authorizers: Vec<fn(&Operation<'_>) -> Result<Option<Decision>, ErrorCode>>,
) -> Result<Option<Decision>, ErrorCode> {
    for authorize in authorizers {
        match authorize(&operation)? {
            None => {}
            Some(Decision::Granted) => return Ok(Some(Decision::Granted)),
            Some(Decision::Denied(error_code)) => return Ok(Some(Decision::Denied(error_code))),
        }
    }
    Ok(None)
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
