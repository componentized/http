//! Helpers for wasi:http.
//!
//! Each component generates its own bindings for `wasi:http/types`, so the helpers for its types
//! are macros, expanded at the call site for the component's type, e.g.
//! `format_http_method!(types::Method, &method)`.

#[doc(hidden)]
pub use std::borrow::Cow;

/// Format a wasi:http `error-code` for logs, as a `Cow<'static, str>`. The name of the variant as
/// written in the WIT, in lower case, e.g. `http-request-denied`. A payload is appended with the
/// values that are present, e.g. `http-response-body-size<1024>` or
/// `dns-error<rcode=NXDOMAIN,info-code=3>`.
///
/// Each component generates its own bindings for `wasi:http/types`, so the match is a macro,
/// expanded at the call site for the component's `error-code` type.
///
/// ```ignore
/// use crate::wasi::http::types::ErrorCode;
///
/// let name = http_utils::format_http_error_code!(ErrorCode, &error_code);
/// // or with a path to the type
/// let name = http_utils::format_http_error_code!(types::ErrorCode, &error_code);
/// ```
#[macro_export]
macro_rules! format_http_error_code {
    ($($error_code:ident)::+, $value:expr) => {
        match $value {
            $($error_code)::+::DnsTimeout => $crate::Cow::Borrowed("dns-timeout"),
            $($error_code)::+::DnsError(payload) => $crate::with_fields(
                "dns-error",
                &[
                    ("rcode", payload.rcode.clone()),
                    ("info-code", payload.info_code.map(|code| code.to_string())),
                ],
            ),
            $($error_code)::+::DestinationNotFound => $crate::Cow::Borrowed("destination-not-found"),
            $($error_code)::+::DestinationUnavailable => $crate::Cow::Borrowed("destination-unavailable"),
            $($error_code)::+::DestinationIpProhibited => {
                $crate::Cow::Borrowed("destination-ip-prohibited")
            }
            $($error_code)::+::DestinationIpUnroutable => {
                $crate::Cow::Borrowed("destination-ip-unroutable")
            }
            $($error_code)::+::ConnectionRefused => $crate::Cow::Borrowed("connection-refused"),
            $($error_code)::+::ConnectionTerminated => $crate::Cow::Borrowed("connection-terminated"),
            $($error_code)::+::ConnectionTimeout => $crate::Cow::Borrowed("connection-timeout"),
            $($error_code)::+::ConnectionReadTimeout => $crate::Cow::Borrowed("connection-read-timeout"),
            $($error_code)::+::ConnectionWriteTimeout => {
                $crate::Cow::Borrowed("connection-write-timeout")
            }
            $($error_code)::+::ConnectionLimitReached => {
                $crate::Cow::Borrowed("connection-limit-reached")
            }
            $($error_code)::+::TlsProtocolError => $crate::Cow::Borrowed("tls-protocol-error"),
            $($error_code)::+::TlsCertificateError => $crate::Cow::Borrowed("tls-certificate-error"),
            $($error_code)::+::TlsAlertReceived(payload) => $crate::with_fields(
                "tls-alert-received",
                &[
                    ("alert-id", payload.alert_id.map(|id| id.to_string())),
                    ("alert-message", payload.alert_message.clone()),
                ],
            ),
            $($error_code)::+::HttpRequestDenied => $crate::Cow::Borrowed("http-request-denied"),
            $($error_code)::+::HttpRequestLengthRequired => {
                $crate::Cow::Borrowed("http-request-length-required")
            }
            $($error_code)::+::HttpRequestBodySize(size) => {
                $crate::with_value("http-request-body-size", size.map(|size| size.to_string()))
            }
            $($error_code)::+::HttpRequestMethodInvalid => {
                $crate::Cow::Borrowed("http-request-method-invalid")
            }
            $($error_code)::+::HttpRequestUriInvalid => $crate::Cow::Borrowed("http-request-uri-invalid"),
            $($error_code)::+::HttpRequestUriTooLong => {
                $crate::Cow::Borrowed("http-request-uri-too-long")
            }
            $($error_code)::+::HttpRequestHeaderSectionSize(size) => $crate::with_value(
                "http-request-header-section-size",
                size.map(|size| size.to_string()),
            ),
            $($error_code)::+::HttpRequestHeaderSize(None) => {
                $crate::Cow::Borrowed("http-request-header-size")
            }
            $($error_code)::+::HttpRequestHeaderSize(Some(payload)) => {
                $crate::with_field_size!("http-request-header-size", payload)
            }
            $($error_code)::+::HttpRequestTrailerSectionSize(size) => $crate::with_value(
                "http-request-trailer-section-size",
                size.map(|size| size.to_string()),
            ),
            $($error_code)::+::HttpRequestTrailerSize(payload) => {
                $crate::with_field_size!("http-request-trailer-size", payload)
            }
            $($error_code)::+::HttpResponseIncomplete => {
                $crate::Cow::Borrowed("http-response-incomplete")
            }
            $($error_code)::+::HttpResponseHeaderSectionSize(size) => $crate::with_value(
                "http-response-header-section-size",
                size.map(|size| size.to_string()),
            ),
            $($error_code)::+::HttpResponseHeaderSize(payload) => {
                $crate::with_field_size!("http-response-header-size", payload)
            }
            $($error_code)::+::HttpResponseBodySize(size) => {
                $crate::with_value("http-response-body-size", size.map(|size| size.to_string()))
            }
            $($error_code)::+::HttpResponseTrailerSectionSize(size) => $crate::with_value(
                "http-response-trailer-section-size",
                size.map(|size| size.to_string()),
            ),
            $($error_code)::+::HttpResponseTrailerSize(payload) => {
                $crate::with_field_size!("http-response-trailer-size", payload)
            }
            $($error_code)::+::HttpResponseTransferCoding(coding) => {
                $crate::with_value("http-response-transfer-coding", coding.clone())
            }
            $($error_code)::+::HttpResponseContentCoding(coding) => {
                $crate::with_value("http-response-content-coding", coding.clone())
            }
            $($error_code)::+::HttpResponseTimeout => $crate::Cow::Borrowed("http-response-timeout"),
            $($error_code)::+::HttpUpgradeFailed => $crate::Cow::Borrowed("http-upgrade-failed"),
            $($error_code)::+::HttpProtocolError => $crate::Cow::Borrowed("http-protocol-error"),
            $($error_code)::+::LoopDetected => $crate::Cow::Borrowed("loop-detected"),
            $($error_code)::+::ConfigurationError => $crate::Cow::Borrowed("configuration-error"),
            $($error_code)::+::InternalError(message) => {
                $crate::with_value("internal-error", message.clone())
            }
        }
    };
}

/// Parse the name of a wasi:http `error-code`, as [`format_http_error_code!`] formats it, e.g.
/// `http-request-denied`, as an `Option<ErrorCode>`. A variant with a payload has an empty payload,
/// `None` for an option and for each field of a record. `None` when the name isn't an error code.
///
/// Takes the bindings' `types` module, rather than the `error-code` type, for the records some
/// payloads are.
///
/// ```ignore
/// let reason = http_utils::parse_http_error_code!(wasi::http::types, "connection-refused");
/// ```
#[macro_export]
macro_rules! parse_http_error_code {
    ($($types:ident)::+, $value:expr) => {
        match $value {
            "dns-timeout" => Some($($types)::+::ErrorCode::DnsTimeout),
            "dns-error" => Some($($types)::+::ErrorCode::DnsError($($types)::+::DnsErrorPayload { rcode: None, info_code: None })),
            "destination-not-found" => Some($($types)::+::ErrorCode::DestinationNotFound),
            "destination-unavailable" => Some($($types)::+::ErrorCode::DestinationUnavailable),
            "destination-ip-prohibited" => Some($($types)::+::ErrorCode::DestinationIpProhibited),
            "destination-ip-unroutable" => Some($($types)::+::ErrorCode::DestinationIpUnroutable),
            "connection-refused" => Some($($types)::+::ErrorCode::ConnectionRefused),
            "connection-terminated" => Some($($types)::+::ErrorCode::ConnectionTerminated),
            "connection-timeout" => Some($($types)::+::ErrorCode::ConnectionTimeout),
            "connection-read-timeout" => Some($($types)::+::ErrorCode::ConnectionReadTimeout),
            "connection-write-timeout" => Some($($types)::+::ErrorCode::ConnectionWriteTimeout),
            "connection-limit-reached" => Some($($types)::+::ErrorCode::ConnectionLimitReached),
            "tls-protocol-error" => Some($($types)::+::ErrorCode::TlsProtocolError),
            "tls-certificate-error" => Some($($types)::+::ErrorCode::TlsCertificateError),
            "tls-alert-received" => Some($($types)::+::ErrorCode::TlsAlertReceived($($types)::+::TlsAlertReceivedPayload { alert_id: None, alert_message: None })),
            "http-request-denied" => Some($($types)::+::ErrorCode::HttpRequestDenied),
            "http-request-length-required" => Some($($types)::+::ErrorCode::HttpRequestLengthRequired),
            "http-request-body-size" => Some($($types)::+::ErrorCode::HttpRequestBodySize(None)),
            "http-request-method-invalid" => Some($($types)::+::ErrorCode::HttpRequestMethodInvalid),
            "http-request-uri-invalid" => Some($($types)::+::ErrorCode::HttpRequestUriInvalid),
            "http-request-uri-too-long" => Some($($types)::+::ErrorCode::HttpRequestUriTooLong),
            "http-request-header-section-size" => Some($($types)::+::ErrorCode::HttpRequestHeaderSectionSize(None)),
            "http-request-header-size" => Some($($types)::+::ErrorCode::HttpRequestHeaderSize(None)),
            "http-request-trailer-section-size" => Some($($types)::+::ErrorCode::HttpRequestTrailerSectionSize(None)),
            "http-request-trailer-size" => Some($($types)::+::ErrorCode::HttpRequestTrailerSize($($types)::+::FieldSizePayload { field_name: None, field_size: None })),
            "http-response-incomplete" => Some($($types)::+::ErrorCode::HttpResponseIncomplete),
            "http-response-header-section-size" => Some($($types)::+::ErrorCode::HttpResponseHeaderSectionSize(None)),
            "http-response-header-size" => Some($($types)::+::ErrorCode::HttpResponseHeaderSize($($types)::+::FieldSizePayload { field_name: None, field_size: None })),
            "http-response-body-size" => Some($($types)::+::ErrorCode::HttpResponseBodySize(None)),
            "http-response-trailer-section-size" => Some($($types)::+::ErrorCode::HttpResponseTrailerSectionSize(None)),
            "http-response-trailer-size" => Some($($types)::+::ErrorCode::HttpResponseTrailerSize($($types)::+::FieldSizePayload { field_name: None, field_size: None })),
            "http-response-transfer-coding" => Some($($types)::+::ErrorCode::HttpResponseTransferCoding(None)),
            "http-response-content-coding" => Some($($types)::+::ErrorCode::HttpResponseContentCoding(None)),
            "http-response-timeout" => Some($($types)::+::ErrorCode::HttpResponseTimeout),
            "http-upgrade-failed" => Some($($types)::+::ErrorCode::HttpUpgradeFailed),
            "http-protocol-error" => Some($($types)::+::ErrorCode::HttpProtocolError),
            "loop-detected" => Some($($types)::+::ErrorCode::LoopDetected),
            "configuration-error" => Some($($types)::+::ErrorCode::ConfigurationError),
            "internal-error" => Some($($types)::+::ErrorCode::InternalError(None)),
            _ => None,
        }
    };
}

/// Format a wasi:http `method` for logs, as a `Cow<'static, str>`. The method in lower case, e.g.
/// `get`, including a method without a variant of its own, e.g. `purge`.
///
/// ```ignore
/// let method = http_utils::format_http_method!(types::Method, &request.get_method());
/// ```
#[macro_export]
macro_rules! format_http_method {
    ($($method:ident)::+, $value:expr) => {
        match $value {
            $($method)::+::Get => $crate::Cow::Borrowed("get"),
            $($method)::+::Head => $crate::Cow::Borrowed("head"),
            $($method)::+::Post => $crate::Cow::Borrowed("post"),
            $($method)::+::Put => $crate::Cow::Borrowed("put"),
            $($method)::+::Delete => $crate::Cow::Borrowed("delete"),
            $($method)::+::Connect => $crate::Cow::Borrowed("connect"),
            $($method)::+::Options => $crate::Cow::Borrowed("options"),
            $($method)::+::Trace => $crate::Cow::Borrowed("trace"),
            $($method)::+::Patch => $crate::Cow::Borrowed("patch"),
            $($method)::+::Other(method) => $crate::Cow::Owned(method.to_ascii_lowercase()),
        }
    };
}

/// Format a wasi:http `scheme` for logs, as a `Cow<'static, str>`. The scheme in lower case, e.g.
/// `https`, including a scheme without a variant of its own, e.g. `ftp`.
///
/// ```ignore
/// let scheme = http_utils::format_http_scheme!(types::Scheme, &scheme);
/// ```
#[macro_export]
macro_rules! format_http_scheme {
    ($($scheme:ident)::+, $value:expr) => {
        match $value {
            $($scheme)::+::Http => $crate::Cow::Borrowed("http"),
            $($scheme)::+::Https => $crate::Cow::Borrowed("https"),
            $($scheme)::+::Other(scheme) => $crate::Cow::Owned(scheme.to_ascii_lowercase()),
        }
    };
}

/// The name with a value, or the name alone without one, e.g. `name<value>`.
#[doc(hidden)]
pub fn with_value(name: &'static str, value: Option<String>) -> Cow<'static, str> {
    match value {
        Some(value) => Cow::Owned(format!("{name}<{value}>")),
        None => Cow::Borrowed(name),
    }
}

/// The name with the fields that have a value, or the name alone without any, e.g.
/// `name<rcode=NXDOMAIN,info-code=3>`.
#[doc(hidden)]
pub fn with_fields(name: &'static str, fields: &[(&str, Option<String>)]) -> Cow<'static, str> {
    let fields: Vec<String> = fields
        .iter()
        .filter_map(|(field, value)| value.as_ref().map(|value| format!("{field}={value}")))
        .collect();
    with_value(name, (!fields.is_empty()).then(|| fields.join(",")))
}

/// The name with a `field-size-payload`, e.g. `name<field-name=x-a,field-size=10>`.
#[doc(hidden)]
#[macro_export]
macro_rules! with_field_size {
    ($name:literal, $payload:expr) => {
        $crate::with_fields(
            $name,
            &[
                ("field-name", $payload.field_name.clone()),
                (
                    "field-size",
                    $payload.field_size.map(|size| size.to_string()),
                ),
            ],
        )
    };
}
