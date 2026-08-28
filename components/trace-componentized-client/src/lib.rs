use std::fmt::Display;

use crate::{
    componentized::http::client::{self, ErrorCode, HttpResponse, Method, RequestOptions},
    exports::componentized::http::client::Guest,
    wasi::logging::logging::{Level, log},
};
use wit_bindgen::StreamReader;

#[macro_export]
macro_rules! trace {
    ($dst:expr, $($arg:tt)*) => {
        log(Level::Trace, "componentized-trace", &format!($dst, $($arg)*));
    };
    ($dst:expr) => {
        log(Level::Trace, "componentized-trace", &format!($dst));
    };
}

struct TraceComponentizedClient;

impl Guest for TraceComponentizedClient {
    async fn request(
        method: Method,
        url: String,
        headers: Vec<(String, String)>,
        body: Option<StreamReader<u8>>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        trace!("OPERATION=componentized:http/client#request URL={url}");
        client::request(method, url, headers, body, options).await
    }

    async fn get(
        url: String,
        headers: Vec<(String, String)>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        trace!("OPERATION=componentized:http/client#get URL={url}");
        client::get(url, headers, options).await
    }

    async fn post(
        url: String,
        headers: Vec<(String, String)>,
        body: StreamReader<u8>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        trace!("OPERATION=componentized:http/client#post URL={url}");
        client::post(url, headers, body, options).await
    }

    async fn put(
        url: String,
        headers: Vec<(String, String)>,
        body: StreamReader<u8>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        trace!("OPERATION=componentized:http/client#put URL={url}");
        client::put(url, headers, body, options).await
    }

    async fn delete(
        url: String,
        headers: Vec<(String, String)>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        trace!("OPERATION=componentized:http/client#delete URL={url}");
        client::delete(url, headers, options).await
    }

    async fn patch(
        url: String,
        headers: Vec<(String, String)>,
        body: StreamReader<u8>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        trace!("OPERATION=componentized:http/client#patch URL={url}");
        client::patch(url, headers, body, options).await
    }

    async fn head(
        url: String,
        headers: Vec<(String, String)>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        trace!("OPERATION=componentized:http/client#head URL={url}");
        client::head(url, headers, options).await
    }

    async fn options(
        url: String,
        headers: Vec<(String, String)>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        trace!("OPERATION=componentized:http/client#options URL={url}");
        client::options(url, headers, options).await
    }

    async fn trace(
        url: String,
        headers: Vec<(String, String)>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        trace!("OPERATION=componentized:http/client#trace URL={url}");
        client::trace(url, headers, options).await
    }

    async fn query(
        url: String,
        headers: Vec<(String, String)>,
        body: StreamReader<u8>,
        options: Option<RequestOptions>,
    ) -> Result<HttpResponse, ErrorCode> {
        trace!("OPERATION=componentized:http/client#query URL={url}");
        client::query(url, headers, body, options).await
    }
}

impl Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Method::Get => f.write_str("get"),
            Method::Post => f.write_str("post"),
            Method::Put => f.write_str("put"),
            Method::Delete => f.write_str("delete"),
            Method::Patch => f.write_str("patch"),
            Method::Head => f.write_str("head"),
            Method::Options => f.write_str("options"),
            Method::Trace => f.write_str("trace"),
            Method::Query => f.write_str("query"),
        }
    }
}
wit_bindgen::generate!({
    path: "../wit",
    world: "trace-componentized-client",
    merge_structurally_equal_types: true,
    generate_all,
});

export!(TraceComponentizedClient);
