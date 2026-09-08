use std::fmt::{self, Display};

use crate::{
    componentized::http::latch::{
        self, ClientOperation,
        Decision::{self, Deferred, Denied},
        ErrorCode, HttpErrorCode, Operation, SendArgs,
    },
    exports::wasi::http::client::{Guest, Request, Response},
    wasi::{
        http::{client, types},
        logging::logging::{Level, log},
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

macro_rules! error {
    ($dst:expr, $($arg:tt)*) => {
        log(Level::Error, "componentized-gate", &format!($dst, $($arg)*));
    };
    ($dst:expr) => {
        log(Level::Error, "componentized-gate", &format!($dst));
    };
}

/// Authorize the request with the latch, then report the decision back to the latch. Latches act
/// on the final decision in `observe-decision`, never while authorizing. Failing to observe the
/// decision fails the request, the same as a latch error.
fn authorize(operation: &Operation) -> Result<Decision, ErrorCode> {
    let decision = latch::authorize(operation)?;
    latch::observe_decision(&decision, operation)?;
    Ok(decision)
}

struct GatedHttpClient {}

impl Guest for GatedHttpClient {
    #[doc = "/ This function may be used to either send an outgoing request over the"]
    #[doc = "/ network or to forward it to another component."]
    #[allow(async_fn_in_trait)]
    async fn send(request: Request) -> Result<Response, HttpErrorCode> {
        let call_summary = || {
            format!(
                "OPERATION=wasi:http/client#send METHOD={method} PATH-WITH-QUERY={path_with_query}",
                method = request.get_method(),
                path_with_query = DisplayOption(request.get_path_with_query()),
            )
        };

        match authorize(&Operation::Client(ClientOperation::Send(SendArgs {
            request: &request,
        }))) {
            Ok(Denied(reason)) => {
                warn!(
                    "Denied REASON={} {}",
                    DisplayReason(&reason),
                    call_summary()
                );
                Err(reason)?
            }
            Ok(Deferred) => client::send(request).await,
            Err(code) => {
                error!(
                    "Latch error CODE={code} {summary}",
                    code = DisplayLatchError(&code),
                    summary = call_summary()
                );
                Err(code)?
            }
        }
    }
}

impl From<ErrorCode> for HttpErrorCode {
    fn from(value: ErrorCode) -> Self {
        match value {
            ErrorCode::InvalidConfig(latch) => {
                Self::InternalError(Some(format!("latch-error: invalid-config<{latch}>")))
            }
            ErrorCode::ObservationFailed(latch) => {
                Self::InternalError(Some(format!("latch-error: observation-failed<{latch}>")))
            }
            ErrorCode::Other(Some(message)) => {
                Self::InternalError(Some(format!("latch-error: {message}")))
            }
            ErrorCode::Other(None) => Self::InternalError(Some("latch-error".to_string())),
        }
    }
}

impl Display for types::Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&http_utils::format_http_method!(types::Method, self))
    }
}

struct DisplayLatchError<'a>(&'a ErrorCode);
impl fmt::Display for DisplayLatchError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self(ErrorCode::InvalidConfig(latch)) => {
                write!(f, "invalid-config<{latch}>")
            }
            Self(ErrorCode::ObservationFailed(latch)) => {
                write!(f, "observation-failed<{latch}>")
            }
            Self(ErrorCode::Other(Some(message))) => f.write_str(message),
            Self(ErrorCode::Other(None)) => f.write_str("other"),
        }
    }
}

/// Displays a denial reason, the name of the error code, e.g. `http-request-denied`.
///
/// The generated bindings already implement `Display` for [`HttpErrorCode`], with its debug form.
struct DisplayReason<'a>(&'a HttpErrorCode);
impl fmt::Display for DisplayReason<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&http_utils::format_http_error_code!(HttpErrorCode, self.0))
    }
}

struct DisplayOption<T>(Option<T>);
impl<T: fmt::Display> fmt::Display for DisplayOption<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(value) => write!(f, "some<{}>", value),
            None => write!(f, "none"), // Customize what to print if empty
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
