//! Virtualizes wasi:http interfaces, logging each call at the TRACE level.
//!
//! These sources are shared by the `trace`, `trace-client`, `trace-handler` and `trace-types`
//! components. Each component compiles them with the features for the interfaces it exports,
//! `wasi:http/types` is always exported:
//!
//! - `client`: exports `wasi:http/client`
//! - `handler`: exports `wasi:http/handler`

use std::fmt::Display;

mod types;

#[cfg(feature = "client")]
mod client;

#[cfg(feature = "handler")]
mod handler;

#[macro_export]
macro_rules! trace {
    ($dst:expr, $($arg:tt)*) => {
        $crate::wasi::logging::logging::log(
            $crate::wasi::logging::logging::Level::Trace,
            "componentized-trace",
            &format!($dst, $($arg)*),
        );
    };
    ($dst:expr) => {
        $crate::wasi::logging::logging::log(
            $crate::wasi::logging::logging::Level::Trace,
            "componentized-trace",
            &format!($dst),
        );
    };
}

/// The number of trailing hex digits of a resource handle to display
const HANDLE_DIGITS: u32 = 4;
const HANDLE_MODULUS: u32 = 16u32.pow(HANDLE_DIGITS);

/// Write the trailing hex digits of a resource handle, padded with zeros and prefixed with `&`,
/// e.g. `&002a`. The prefix marks the value as a handle, a reference to a resource.
fn fmt_handle(f: &mut std::fmt::Formatter<'_>, handle: u32) -> std::fmt::Result {
    write!(
        f,
        "&{:0width$x}",
        handle % HANDLE_MODULUS,
        width = HANDLE_DIGITS as usize
    )
}

struct DisplayOption<T>(Option<T>);

impl<T: Display> Display for DisplayOption<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self(Some(value)) => write!(f, "some<{value}>"),
            Self(None) => f.write_str("none"),
        }
    }
}

struct Trace;

#[cfg(all(feature = "client", feature = "handler"))]
wit_bindgen::generate!({
    // `path` is relative to the manifest of the component compiling these sources, `components/<name>/`
    path: "../wit",
    world: "trace",
    merge_structurally_equal_types: true,
    generate_all,
});

#[cfg(all(feature = "client", not(feature = "handler")))]
wit_bindgen::generate!({
    // `path` is relative to the manifest of the component compiling these sources, `components/<name>/`
    path: "../wit",
    world: "trace-client",
    merge_structurally_equal_types: true,
    generate_all,
});

#[cfg(all(not(feature = "client"), feature = "handler"))]
wit_bindgen::generate!({
    // `path` is relative to the manifest of the component compiling these sources, `components/<name>/`
    path: "../wit",
    world: "trace-handler",
    merge_structurally_equal_types: true,
    generate_all,
});

#[cfg(all(not(feature = "client"), not(feature = "handler")))]
wit_bindgen::generate!({
    // `path` is relative to the manifest of the component compiling these sources, `components/<name>/`
    path: "../wit",
    world: "trace-types",
    merge_structurally_equal_types: true,
    generate_all,
});

export!(Trace);
