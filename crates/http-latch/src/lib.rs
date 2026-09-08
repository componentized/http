//! Bindings and helpers for latch components, which implement the `latch` world.
//!
//! The bindings are generated once here and shared by every latch component. A latch implements
//! [`Latch`] and exports it with [`export!`]:
//!
//! ```ignore
//! struct MyLatch {}
//!
//! impl http_latch::Latch for MyLatch {
//!     // ...
//! }
//!
//! http_latch::export!(MyLatch with_types_in http_latch::bindings);
//! ```
//!
//! Imports a latch does not use, like `wasi:config/store` or the wrapped latch, are dropped from
//! the built component.

use core::cell::RefCell;
use core::fmt;
use core::ops::Deref;
use std::collections::HashMap;

pub mod bindings {
    wit_bindgen::generate!({
        path: "../../components/wit",
        world: "latch",
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

pub use bindings::exports::componentized::http::latch::{
    ClientOperation, Decision, ErrorCode, Guest as Latch, HandleArgs, HandlerOperation,
    HttpErrorCode, Operation, SendArgs,
};
pub use bindings::wasi::http::types::{Method, Request, Scheme};

/// The latch a wrapping latch delegates to, imported as `componentized:http/latch`.
pub mod wrapped {
    pub use crate::bindings::componentized::http::latch::{authorize, observe_decision};
}

/// Log a message from a latch with `wasi:logging`.
pub fn log(level: bindings::wasi::logging::logging::Level, message: &str) {
    bindings::wasi::logging::logging::log(level, "componentized-latch", message);
}

/// Log a critical message from a latch, formatted like `format!`.
#[macro_export]
macro_rules! critical {
    ($($arg:tt)*) => {
        $crate::log($crate::bindings::wasi::logging::logging::Level::Critical, &format!($($arg)*))
    };
}

/// Log an error from a latch, formatted like `format!`.
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::log($crate::bindings::wasi::logging::logging::Level::Error, &format!($($arg)*))
    };
}

/// Log a warning from a latch, formatted like `format!`.
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::log($crate::bindings::wasi::logging::logging::Level::Warn, &format!($($arg)*))
    };
}

/// Log a trace message from a latch, formatted like `format!`.
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        $crate::log($crate::bindings::wasi::logging::logging::Level::Trace, &format!($($arg)*))
    };
}

/// Load the latch's config from `wasi:config/store` and parse it.
///
/// When the config cannot be read or parsed, the cause is logged and an `invalid-config` error
/// naming the latch is returned. Parse errors describe the offending entry, e.g.
/// `KEY=<key> VALUE=<value> ERROR=<error>`.
pub fn load_config<T>(
    latch_name: &str,
    parse: impl FnOnce(Vec<(String, String)>) -> Result<T, String>,
) -> Result<T, ErrorCode> {
    use bindings::wasi::config::store;

    store::get_all()
        .map_err(|err| match err {
            store::Error::Upstream(message) => format!("ERROR=upstream: {message}"),
            store::Error::Io(message) => format!("ERROR=io: {message}"),
        })
        .and_then(parse)
        .map_err(|message| {
            critical!("Invalid config LATCH={latch_name} {message}");
            ErrorCode::InvalidConfig(latch_name.to_string())
        })
}

/// A decision configured for each key, e.g. `get=deferred` and `*=denied`.
///
/// Values are `deferred`, or empty, and `denied`. A key that isn't configured falls back to `*`,
/// and is deferred without it.
pub struct DecisionConfig {
    denied: HashMap<String, bool>,
}

/// The key a decision falls back to.
pub const WILDCARD: &str = "*";

impl DecisionConfig {
    /// Parse the config entries, for [`load_config`].
    pub fn parse(entries: Vec<(String, String)>) -> Result<Self, String> {
        let mut denied = HashMap::new();
        for (key, value) in entries {
            let is_denied = match value.as_str() {
                "" | "deferred" => false,
                "denied" => true,
                _ => {
                    return Err(format!(
                        "KEY={key} VALUE={value} ERROR=expected one of: 'deferred', 'denied'"
                    ));
                }
            };
            denied.insert(key, is_denied);
        }
        Ok(Self { denied })
    }

    /// The decision for the key, denied with the reason.
    pub fn decide(&self, key: &str, reason: impl FnOnce() -> HttpErrorCode) -> Decision {
        let denied = self
            .denied
            .get(key)
            .or_else(|| self.denied.get(WILDCARD))
            .copied()
            .unwrap_or(false);
        match denied {
            true => Decision::Denied(reason()),
            false => Decision::Deferred,
        }
    }
}

/// State kept by a latch between calls, in a `static`.
///
/// Components are single threaded, and a component is not reentered while it is running, so the
/// state is never shared between threads.
pub struct Local<T>(RefCell<T>);

// components are single threaded, and a component is not reentered while it is running
unsafe impl<T> Sync for Local<T> {}

impl<T> Local<T> {
    pub const fn new(value: T) -> Local<T> {
        Local(RefCell::new(value))
    }
}

impl<T> Deref for Local<T> {
    type Target = RefCell<T>;

    fn deref(&self) -> &RefCell<T> {
        &self.0
    }
}

/// Displays a latch error the way gates log it, e.g. `invalid-config<latch-name>`.
///
/// The generated bindings already implement `Display` for [`ErrorCode`], with its debug form.
pub struct DisplayError<'a>(pub &'a ErrorCode);

impl fmt::Display for DisplayError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            ErrorCode::InvalidConfig(latch) => write!(f, "invalid-config<{latch}>"),
            ErrorCode::ObservationFailed(latch) => write!(f, "observation-failed<{latch}>"),
            ErrorCode::Other(Some(message)) => f.write_str(message),
            ErrorCode::Other(None) => f.write_str("other"),
        }
    }
}

/// Displays a denial reason the way gates log it, the name of the error code, e.g.
/// `http-request-denied`.
///
/// The generated bindings already implement `Display` for [`HttpErrorCode`], with its debug form.
pub struct DisplayReason<'a>(pub &'a HttpErrorCode);

impl fmt::Display for DisplayReason<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&http_utils::format_http_error_code!(HttpErrorCode, self.0))
    }
}

/// Displays a decision, e.g. `DECISION=denied REASON=http-request-denied`.
pub struct DisplayDecision<'a>(pub &'a Decision);

impl fmt::Display for DisplayDecision<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Decision::Deferred => f.write_str("DECISION=deferred"),
            Decision::Denied(reason) => {
                write!(f, "DECISION=denied REASON={}", DisplayReason(reason))
            }
        }
    }
}

/// A method, displayed the way gates log it, e.g. `get`.
impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&http_utils::format_http_method!(Method, self))
    }
}

/// A scheme, displayed the way gates log it, e.g. `https`.
impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&http_utils::format_http_scheme!(Scheme, self))
    }
}

/// The request an operation is for.
pub fn request<'a>(operation: &Operation<'a>) -> &'a Request {
    match operation {
        Operation::Client(ClientOperation::Send(args)) => args.request,
        Operation::Handler(HandlerOperation::Handle(args)) => args.request,
    }
}

/// An operation, displayed the way gates log it, e.g.
/// `OPERATION=wasi:http/client#send METHOD=get PATH-WITH-QUERY=some</>`.
impl fmt::Display for Operation<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let operation = match self {
            Operation::Client(ClientOperation::Send(_)) => "wasi:http/client#send",
            Operation::Handler(HandlerOperation::Handle(_)) => "wasi:http/handler#handle",
        };
        let request = request(self);
        write!(f, "OPERATION={operation} METHOD={}", request.get_method())?;
        match request.get_path_with_query() {
            Some(path_with_query) => write!(f, " PATH-WITH-QUERY=some<{path_with_query}>"),
            None => f.write_str(" PATH-WITH-QUERY=none"),
        }
    }
}
