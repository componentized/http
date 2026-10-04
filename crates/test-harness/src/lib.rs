//! Test harness for the trace components.
//!
//! A [`Harness`] instantiates a component (`target/components/*/*.wasm`) with `wasi:http` from
//! wasmtime-wasi-http and captured `wasi:logging` output. Requests the component sends upstream,
//! with either `wasi:http/client` or `wasi:http/handler`, never reach the network, they are
//! recorded and answered with a canned response.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::task::Poll;

use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use tokio::sync::{mpsc, oneshot};
use wasmtime::component::{
    Accessor, Component, FutureConsumer, FutureReader, HasData, HasSelf, Lift, Linker, Lower,
    Resource, ResourceTable, Source, StreamConsumer, StreamReader, StreamResult,
};
use wasmtime::error::Context as _;
use wasmtime::{Config, Engine, Result, Store, StoreContextMut, bail, format_err};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};
use wasmtime_wasi_http::p3::bindings::http::client::HostWithStore as _;
use wasmtime_wasi_http::{
    RequestOptions, WasiBody, WasiHttp, WasiHttpCtx, WasiHttpCtxView, WasiHttpHooks, WasiHttpView,
};

use crate::bindings::componentized::http::client as upstream_client;
use crate::bindings::exports::componentized::http::client as http_client;
use crate::bindings::exports::wasi::http::{client, handler, types};
use crate::bindings::wasi::logging::logging;
use crate::gate_bindings::exports::wasi::http as gated;

mod latch;

pub use latch::{Authorization, HostLatch, Observation, host_request, response_status};

/// Bindings for gates, which export `wasi:http/client` and `wasi:http/handler` with the host's
/// `wasi:http/types`, and import a latch.
pub mod gate_bindings {
    wasmtime::component::bindgen!({
        path: "../../components/wit",
        inline: "
            package componentized:test-harness-gate;

            world gate {
                import componentized:http/latch@0.1.0-dev;
                import wasi:config/store@0.2.0-rc.1;
                import wasi:logging/logging@0.1.0-draft;
                export wasi:http/client@0.3.0;
                export wasi:http/handler@0.3.0;
            }
        ",
        world: "componentized:test-harness-gate/gate",
        exports: { default: async | store },
        with: {
            "wasi:http/types": wasmtime_wasi_http::p3::bindings::http::types,
            "wasi:clocks": wasmtime_wasi::p3::bindings::clocks,
            "wasi:logging": crate::bindings::wasi::logging,
        },
    });
}

pub use gate_bindings::componentized::http::latch::{
    Decision, ErrorCode as LatchErrorCode, HttpErrorCode,
};

pub mod bindings {
    wasmtime::component::bindgen!({
        path: "../../components/wit",
        inline: "
            package componentized:test-harness;

            world harness {
                import wasi:logging/logging@0.1.0-draft;
                import componentized:http/client@0.1.0-dev;
                export componentized:http/client@0.1.0-dev;
                export wasi:http/types@0.3.0;
                export wasi:http/client@0.3.0;
                export wasi:http/handler@0.3.0;
            }
        ",
        world: "componentized:test-harness/harness",
        imports: {
            "componentized:http/client": async | store,
        },
        exports: { default: async | store },
        with: {
            "wasi:clocks": wasmtime_wasi::p3::bindings::clocks,
        },
    });
}

pub use bindings::exports::componentized::http::client::{
    ErrorCode as HttpClientErrorCode, HttpResponse as HttpClientResponse,
    Method as HttpClientMethod, RedirectRequest as HttpClientRedirectRequest,
    RequestOptions as HttpClientRequestOptions,
};
pub use bindings::exports::wasi::http::types::{ErrorCode, Method, Scheme};
pub use bindings::wasi::logging::logging::Level;
pub use wasmtime::component::ResourceAny;

/// The status of the canned response to requests sent upstream.
pub const UPSTREAM_STATUS: u16 = 200;

/// A header of the canned response to requests sent upstream, `(name, value)`.
pub const UPSTREAM_HEADER: (&str, &str) = ("x-upstream", "canned");

/// The number of trailing hex digits of a resource handle the trace components log.
const HANDLE_DIGITS: usize = 4;

/// The prefix marking a logged value as a resource handle.
const HANDLE_PREFIX: char = '&';

/// The trailing hex digits of a logged resource handle, e.g. `00a3` for `&00a3`, `None` if the
/// value is not a handle.
fn handle_digits(value: &str) -> Option<&str> {
    let digits = value.strip_prefix(HANDLE_PREFIX)?;
    let is_hex = digits
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    (digits.len() == HANDLE_DIGITS && is_hex).then_some(digits)
}

/// Replace the resource handles in a trace message, e.g. `SELF=&00a3` becomes `SELF=&----`.
///
/// Handles are assigned by the runtime, masking them keeps assertions about what was logged
/// independent of the order resources were created and dropped.
pub fn mask_handles(message: &str) -> String {
    message
        .split(' ')
        .map(|part| match part.split_once('=') {
            Some((key, value)) if handle_digits(value).is_some() => {
                format!("{key}={HANDLE_PREFIX}{}", "-".repeat(HANDLE_DIGITS))
            }
            _ => part.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A future for the guest that resolves to the value.
pub fn ready<T: Lower + Lift + Send + Sync + 'static>(
    accessor: &Accessor<Ctx>,
    value: T,
) -> Result<FutureReader<T>> {
    accessor.with(|store| FutureReader::new(store, async move { Ok::<_, wasmtime::Error>(value) }))
}

/// A stream for the guest that yields the items, then closes.
pub fn stream<T: Lower + Lift + Unpin + Send + Sync + 'static>(
    accessor: &Accessor<Ctx>,
    items: Vec<T>,
) -> Result<StreamReader<T>> {
    accessor.with(|store| StreamReader::new(store, items))
}

/// Read every item written to a guest stream, until it closes.
pub async fn collect<T: Lift + Send + Sync + 'static>(
    accessor: &Accessor<Ctx>,
    stream: StreamReader<T>,
) -> Result<Vec<T>> {
    let (tx, mut rx) = mpsc::unbounded_channel();
    accessor.with(|store| stream.pipe(store, ChannelConsumer(tx)))?;
    let mut items = vec![];
    while let Some(item) = rx.recv().await {
        items.push(item);
    }
    Ok(items)
}

/// Wait for the value of a guest future.
///
/// Only for futures the guest writes itself. A future the guest passes through from a
/// wasmtime-wasi-http host import is transferred host to host, which requires the same Rust type on
/// both ends, and the harness' exported types are generated separately from wasmtime-wasi-http's.
pub async fn resolve<T: Lift + Send + Sync + 'static>(
    accessor: &Accessor<Ctx>,
    future: FutureReader<T>,
) -> Result<T> {
    let (tx, rx) = oneshot::channel();
    accessor.with(|store| future.pipe(store, OneshotConsumer(Some(tx))))?;
    rx.await
        .map_err(|_| format_err!("future closed without a value"))
}

struct OneshotConsumer<T>(Option<oneshot::Sender<T>>);

impl<D, T: Lift + Send + Sync + 'static> FutureConsumer<D> for OneshotConsumer<T> {
    type Item = T;

    fn poll_consume(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        store: StoreContextMut<D>,
        mut source: Source<'_, T>,
        _finish: bool,
    ) -> Poll<Result<()>> {
        let mut item = None;
        source.read(store, &mut item)?;
        if let (Some(item), Some(tx)) = (item, self.get_mut().0.take()) {
            // the receiver is only dropped when the test stopped waiting
            let _ = tx.send(item);
        }
        Poll::Ready(Ok(()))
    }
}

struct ChannelConsumer<T>(mpsc::UnboundedSender<T>);

impl<D, T: Lift + Send + Sync + 'static> StreamConsumer<D> for ChannelConsumer<T> {
    type Item = T;

    fn poll_consume(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        store: StoreContextMut<D>,
        mut source: Source<'_, T>,
        _finish: bool,
    ) -> Poll<Result<StreamResult>> {
        let mut item = None;
        source.read(store, &mut item)?;
        if let Some(item) = item {
            if self.0.send(item).is_err() {
                return Poll::Ready(Ok(StreamResult::Dropped));
            }
        }
        Poll::Ready(Ok(StreamResult::Completed))
    }
}

/// Root of the workspace, where the Makefile lives.
fn workspace_dir() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    dir.canonicalize().unwrap_or(dir)
}

/// Path containing the built component.
pub fn component_path(name: &str) -> PathBuf {
    workspace_dir().join(format!("target/components/{name}/{name}.wasm"))
}

/// Rebuild the named component in `target/components/` with make, unless it was already built
/// by this process.
fn ensure_built(name: &str) -> Result<()> {
    static BUILT: Mutex<Option<HashSet<String>>> = Mutex::new(None);

    // hold the lock while building so concurrent tests don't run make over each other
    let mut built = BUILT.lock().unwrap_or_else(|err| err.into_inner());
    let built = built.get_or_insert_with(HashSet::new);
    if built.contains(name) {
        return Ok(());
    }

    // the Makefile's targets are relative to the workspace, make matches them textually
    let target = format!("target/components/{name}/{name}.wasm");
    let output = Command::new("make")
        .arg("-C")
        .arg(workspace_dir())
        .arg(&target)
        .output()
        .context("failed to run make")?;
    if !output.status.success() {
        bail!(
            "failed to build {target}:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    built.insert(name.to_string());
    Ok(())
}

/// A message logged by the test subject via `wasi:logging`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {
    pub level: Level,
    pub context: String,
    pub message: String,
}

impl LogEntry {
    /// A warning logged by the test subject.
    pub fn warn(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level: Level::Warn,
            context: context.into(),
            message: message.into(),
        }
    }

    /// An error logged by the test subject.
    pub fn error(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level: Level::Error,
            context: context.into(),
            message: message.into(),
        }
    }

    /// A critical message logged by the test subject.
    pub fn critical(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level: Level::Critical,
            context: context.into(),
            message: message.into(),
        }
    }

    /// A trace message logged by the test subject.
    pub fn trace(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level: Level::Trace,
            context: context.into(),
            message: message.into(),
        }
    }

    /// The entry with the resource handles in its message masked, see [`mask_handles`].
    pub fn masked(self) -> Self {
        Self {
            message: mask_handles(&self.message),
            ..self
        }
    }
}

/// A resource handle in a trace message, e.g. `SELF=&00a3`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoggedHandle {
    /// The traced operation, e.g. `wasi:http/types#request.get-method`
    pub operation: String,
    /// The name the handle is logged under, e.g. `SELF`
    pub key: String,
    /// The handle as logged, e.g. `&00a3`
    pub value: String,
}

/// The resource handles in a trace message.
pub fn logged_handles(message: &str) -> Vec<LoggedHandle> {
    let mut parts = message.split(' ');
    let Some(operation) = parts
        .next()
        .and_then(|part| part.strip_prefix("OPERATION="))
    else {
        return vec![];
    };
    parts
        .filter_map(|part| part.split_once('='))
        .filter(|(_, value)| handle_digits(value).is_some())
        .map(|(key, value)| LoggedHandle {
            operation: operation.to_string(),
            key: key.to_string(),
            value: value.to_string(),
        })
        .collect()
}

/// A request the test subject sent upstream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SentRequest {
    pub method: String,
    pub uri: String,
}

/// A request the test subject sent upstream, with its headers and body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpstreamRequest {
    pub method: String,
    pub uri: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl UpstreamRequest {
    /// The values of the named header, ignoring case.
    pub fn header(&self, name: &str) -> Vec<&str> {
        self.headers
            .iter()
            .filter(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
            .collect()
    }

    /// The path and query of the request URI.
    pub fn path(&self) -> &str {
        self.uri
            .parse::<http::Uri>()
            .ok()
            .and_then(|uri| uri.path_and_query().map(|p| p.as_str().len()))
            .map(|len| &self.uri[self.uri.len() - len..])
            .unwrap_or(&self.uri)
    }
}

/// The response upstream sends for a request, the body is empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpstreamResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
}

impl Default for UpstreamResponse {
    /// [`UPSTREAM_STATUS`] with [`UPSTREAM_HEADER`].
    fn default() -> Self {
        Self {
            status: UPSTREAM_STATUS,
            headers: vec![(UPSTREAM_HEADER.0.to_string(), UPSTREAM_HEADER.1.to_string())],
        }
    }
}

impl UpstreamResponse {
    /// A redirect to the location.
    pub fn redirect(status: u16, location: &str) -> Self {
        Self {
            status,
            headers: vec![("location".to_string(), location.to_string())],
        }
    }
}

type Responder = dyn FnMut(&UpstreamRequest) -> UpstreamResponse + Send;

/// Observations recorded while the test subject runs.
#[derive(Clone, Default)]
pub struct Recorder {
    logs: Arc<Mutex<Vec<LogEntry>>>,
    requests: Arc<Mutex<Vec<UpstreamRequest>>>,
    authorizations: Arc<Mutex<Vec<Authorization>>>,
    observations: Arc<Mutex<Vec<Observation>>>,
}

impl Recorder {
    /// Messages logged by the test subject, in order.
    pub fn logs(&self) -> Vec<LogEntry> {
        self.logs.lock().unwrap().clone()
    }

    /// Messages logged by the test subject, in order, with resource handles masked.
    pub fn masked_logs(&self) -> Vec<LogEntry> {
        self.logs().into_iter().map(LogEntry::masked).collect()
    }

    /// Resource handles in the messages logged by the test subject, in order.
    pub fn handles(&self) -> Vec<LoggedHandle> {
        self.logs()
            .iter()
            .flat_map(|log| logged_handles(&log.message))
            .collect()
    }

    /// Values of the resource handles logged by operations starting with the prefix, in order,
    /// e.g. `wasi:http/types#request.` for every request method.
    pub fn handle_values(&self, operation_prefix: &str) -> Vec<String> {
        self.handles()
            .into_iter()
            .filter(|handle| handle.operation.starts_with(operation_prefix))
            .map(|handle| handle.value)
            .collect()
    }

    /// Requests the host latch was asked to authorize, in order.
    pub fn authorizations(&self) -> Vec<Authorization> {
        self.authorizations.lock().unwrap().clone()
    }

    /// Names of the operations the host latch was asked to authorize, in order, e.g.
    /// `client.send`.
    pub fn operations(&self) -> Vec<String> {
        self.authorizations()
            .into_iter()
            .map(|a| a.operation)
            .collect()
    }

    /// Decisions the host latch observed, in order.
    pub fn observations(&self) -> Vec<Observation> {
        self.observations.lock().unwrap().clone()
    }

    /// Requests the test subject sent upstream, in order.
    pub fn requests(&self) -> Vec<SentRequest> {
        self.upstream_requests()
            .into_iter()
            .map(|request| SentRequest {
                method: request.method,
                uri: request.uri,
            })
            .collect()
    }

    /// Requests the test subject sent upstream, in order, with their headers and body.
    pub fn upstream_requests(&self) -> Vec<UpstreamRequest> {
        self.requests.lock().unwrap().clone()
    }
}

/// Answers requests sent upstream instead of the network, by default with a canned response.
struct Upstream {
    recorder: Recorder,
    responder: Arc<Mutex<Box<Responder>>>,
}

impl WasiHttpHooks for Upstream {
    fn send_request(
        &mut self,
        request: http::Request<WasiBody>,
        _options: Option<RequestOptions>,
        _fut: Box<dyn Future<Output = Result<(), wasmtime_wasi_http::Error>> + Send>,
    ) -> Box<
        dyn Future<
                Output = Result<
                    (
                        http::Response<WasiBody>,
                        Box<dyn Future<Output = Result<(), wasmtime_wasi_http::Error>> + Send>,
                    ),
                    wasmtime_wasi_http::Error,
                >,
            > + Send,
    > {
        let recorder = self.recorder.clone();
        let responder = self.responder.clone();
        Box::new(async move {
            let (parts, body) = request.into_parts();
            let body = body.collect().await?.to_bytes().to_vec();
            let request = UpstreamRequest {
                method: parts.method.to_string(),
                uri: parts.uri.to_string(),
                headers: parts
                    .headers
                    .iter()
                    .map(|(name, value)| {
                        (
                            name.to_string(),
                            String::from_utf8_lossy(value.as_bytes()).into_owned(),
                        )
                    })
                    .collect(),
                body,
            };
            let upstream = (responder.lock().unwrap())(&request);
            recorder.requests.lock().unwrap().push(request);

            // the body is passed through the test subject to the harness host to host, which the
            // harness can't read, see `resolve`
            let body = Empty::<Bytes>::new()
                .map_err(|never| match never {})
                .boxed_unsync();
            let mut response = http::Response::builder().status(upstream.status);
            for (name, value) in &upstream.headers {
                response = response.header(name, value);
            }
            let response = response.body(body).expect("upstream response");
            let io = Box::new(async { Ok(()) })
                as Box<dyn Future<Output = Result<(), wasmtime_wasi_http::Error>> + Send>;
            Ok((response, io))
        })
    }
}

pub struct Ctx {
    wasi: WasiCtx,
    http: WasiHttpCtx,
    upstream: Upstream,
    table: ResourceTable,
    recorder: Recorder,
    latch: HostLatch,
    config: Vec<(String, String)>,
}

impl WasiView for Ctx {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl WasiHttpView for Ctx {
    fn http(&mut self) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            ctx: &mut self.http,
            table: &mut self.table,
            hooks: &mut self.upstream,
        }
    }
}

impl logging::Host for Ctx {
    fn log(&mut self, level: Level, context: String, message: String) {
        self.recorder.logs.lock().unwrap().push(LogEntry {
            level,
            context,
            message,
        });
    }
}

/// Upstream for `componentized:http/client`, requests are recorded and answered with a canned
/// response.
struct UpstreamClient;

impl HasData for UpstreamClient {
    type Data<'a> = &'a mut Ctx;
}

impl upstream_client::Host for Ctx {}

impl UpstreamClient {
    fn respond<T: 'static>(
        accessor: &Accessor<T, Self>,
        method: &str,
        url: String,
        body: Option<StreamReader<u8>>,
    ) -> Result<upstream_client::HttpResponse, upstream_client::ErrorCode> {
        accessor.with(|mut store| {
            store
                .get()
                .recorder
                .requests
                .lock()
                .unwrap()
                .push(UpstreamRequest {
                    method: method.to_string(),
                    uri: url,
                    // not read, the body stream is closed
                    headers: vec![],
                    body: vec![],
                });
            if let Some(mut body) = body {
                body.close(&mut store)
                    .map_err(|err| upstream_client::ErrorCode::Other(Some(err.to_string())))?;
            }
            let response_body = StreamReader::new(&mut store, Vec::<u8>::new())
                .map_err(|err| upstream_client::ErrorCode::Other(Some(err.to_string())))?;
            let trailers =
                FutureReader::new(&mut store, async { Ok::<_, wasmtime::Error>(Ok(vec![])) })
                    .map_err(|err| upstream_client::ErrorCode::Other(Some(err.to_string())))?;
            Ok(upstream_client::HttpResponse {
                status: UPSTREAM_STATUS,
                headers: vec![(UPSTREAM_HEADER.0.to_string(), UPSTREAM_HEADER.1.to_string())],
                body: response_body,
                trailers,
            })
        })
    }
}

impl<T: 'static> upstream_client::HostWithStore<T> for UpstreamClient {
    async fn request(
        accessor: &Accessor<T, Self>,
        method: upstream_client::Method,
        url: String,
        _headers: Vec<(String, String)>,
        body: Option<StreamReader<u8>>,
        _options: Option<upstream_client::RequestOptions>,
    ) -> Result<upstream_client::HttpResponse, upstream_client::ErrorCode> {
        use upstream_client::Method;
        let method = match method {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Patch => "PATCH",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
            Method::Trace => "TRACE",
            Method::Query => "QUERY",
        };
        Self::respond(accessor, method, url, body)
    }

    async fn get(
        accessor: &Accessor<T, Self>,
        url: String,
        _headers: Vec<(String, String)>,
        _options: Option<upstream_client::RequestOptions>,
    ) -> Result<upstream_client::HttpResponse, upstream_client::ErrorCode> {
        Self::respond(accessor, "GET", url, None)
    }

    async fn post(
        accessor: &Accessor<T, Self>,
        url: String,
        _headers: Vec<(String, String)>,
        body: StreamReader<u8>,
        _options: Option<upstream_client::RequestOptions>,
    ) -> Result<upstream_client::HttpResponse, upstream_client::ErrorCode> {
        Self::respond(accessor, "POST", url, Some(body))
    }

    async fn put(
        accessor: &Accessor<T, Self>,
        url: String,
        _headers: Vec<(String, String)>,
        body: StreamReader<u8>,
        _options: Option<upstream_client::RequestOptions>,
    ) -> Result<upstream_client::HttpResponse, upstream_client::ErrorCode> {
        Self::respond(accessor, "PUT", url, Some(body))
    }

    async fn delete(
        accessor: &Accessor<T, Self>,
        url: String,
        _headers: Vec<(String, String)>,
        _options: Option<upstream_client::RequestOptions>,
    ) -> Result<upstream_client::HttpResponse, upstream_client::ErrorCode> {
        Self::respond(accessor, "DELETE", url, None)
    }

    async fn patch(
        accessor: &Accessor<T, Self>,
        url: String,
        _headers: Vec<(String, String)>,
        body: StreamReader<u8>,
        _options: Option<upstream_client::RequestOptions>,
    ) -> Result<upstream_client::HttpResponse, upstream_client::ErrorCode> {
        Self::respond(accessor, "PATCH", url, Some(body))
    }

    async fn head(
        accessor: &Accessor<T, Self>,
        url: String,
        _headers: Vec<(String, String)>,
        _options: Option<upstream_client::RequestOptions>,
    ) -> Result<upstream_client::HttpResponse, upstream_client::ErrorCode> {
        Self::respond(accessor, "HEAD", url, None)
    }

    async fn options(
        accessor: &Accessor<T, Self>,
        url: String,
        _headers: Vec<(String, String)>,
        _options: Option<upstream_client::RequestOptions>,
    ) -> Result<upstream_client::HttpResponse, upstream_client::ErrorCode> {
        Self::respond(accessor, "OPTIONS", url, None)
    }

    async fn trace(
        accessor: &Accessor<T, Self>,
        url: String,
        _headers: Vec<(String, String)>,
        _options: Option<upstream_client::RequestOptions>,
    ) -> Result<upstream_client::HttpResponse, upstream_client::ErrorCode> {
        Self::respond(accessor, "TRACE", url, None)
    }

    async fn query(
        accessor: &Accessor<T, Self>,
        url: String,
        _headers: Vec<(String, String)>,
        body: StreamReader<u8>,
        _options: Option<upstream_client::RequestOptions>,
    ) -> Result<upstream_client::HttpResponse, upstream_client::ErrorCode> {
        Self::respond(accessor, "QUERY", url, Some(body))
    }
}

/// Define `wasi:http/handler` with the same implementation as `wasi:http/client`, the request is
/// sent upstream.
///
/// wasmtime-wasi-http only defines `wasi:http/client`, the two have the same signature.
fn add_handler_to_linker(linker: &mut Linker<Ctx>) -> Result<()> {
    type Request = wasmtime_wasi_http::p3::Request;
    type Response = wasmtime_wasi_http::p3::Response;
    type HttpErrorCode = wasmtime_wasi_http::p3::bindings::http::types::ErrorCode;

    linker
        .instance("wasi:http/handler@0.3.0")?
        .func_wrap_concurrent(
            "handle",
            |accessor: &Accessor<Ctx>, (request,): (Resource<Request>,)| {
                Box::pin(async move {
                    let accessor = accessor.with_getter::<WasiHttp>(<Ctx as WasiHttpView>::http);
                    let result: Result<Resource<Response>, HttpErrorCode> =
                        match WasiHttp::send(&accessor, request).await {
                            Ok(response) => Ok(response),
                            Err(err) => Err(err.downcast()?),
                        };
                    Ok((result,))
                })
            },
        )
}

/// Builds a subject instance for a test.
pub struct Harness {
    subject: String,
    responder: Box<Responder>,
    latches: Vec<String>,
    host_latch: Option<HostLatch>,
    config: Vec<(String, String)>,
}

impl Harness {
    /// Test the named component from `target/components/`, e.g. `trace-types`.
    pub fn new(component_name: &str) -> Self {
        Self {
            subject: component_name.to_string(),
            responder: Box::new(|_| UpstreamResponse::default()),
            latches: vec![],
            host_latch: None,
            config: vec![],
        }
    }

    /// Install a latch component from `target/components/`.
    ///
    /// Without latch components the host latch is the test subject's latch. A single latch
    /// component replaces it, unless the latch imports a latch, then it wraps the host latch.
    /// Otherwise several latches, including the host latch when one is set with
    /// [`Harness::host_latch`], are aggregated with the `latch-n` component of the same size,
    /// in the order installed with the host latch last.
    pub fn latch(mut self, name: &str) -> Self {
        self.latches.push(name.to_string());
        self
    }

    /// Replace the default deferring host latch, it is aggregated with any latch components.
    pub fn host_latch(mut self, latch: HostLatch) -> Self {
        self.host_latch = Some(latch);
        self
    }

    /// Add a `wasi:config/store` value, visible to every component in the composition.
    ///
    /// Values are returned by `get-all` in the order they are added.
    pub fn config(mut self, key: &str, value: &str) -> Self {
        self.config.push((key.to_string(), value.to_string()));
        self
    }

    fn compose(&self) -> Result<Vec<u8>> {
        let mut names = vec![self.subject.as_str()];
        names.extend(self.latches.iter().map(String::as_str));
        for name in &names {
            ensure_built(name)?;
        }

        let read = |name: &str| {
            let path = component_path(name);
            std::fs::read(&path).with_context(|| format!("failed to read {}", path.display()))
        };

        let bytes = read(&self.subject)?;
        let latch = match self.latches.as_slice() {
            [] => return Ok(bytes),
            // a latch that wraps another latch wraps the host latch, its import is left for the host
            [latch] if self.host_latch.is_none() || latch::imports_latch(&read(latch)?)? => {
                read(latch)?
            }
            latches => {
                // the host latch takes the last slot of latch-n, left unsatisfied it is imported
                let slots = latches.len() + usize::from(self.host_latch.is_some());
                if slots > latch::LATCH_N_MAX {
                    bail!(
                        "at most {} latches can be aggregated, got {slots}",
                        latch::LATCH_N_MAX
                    );
                }
                let latch_n = format!("latch-n{slots}");
                ensure_built(&latch_n)?;
                let latches = latches
                    .iter()
                    .map(|name| read(name))
                    .collect::<Result<Vec<_>>>()?;
                latch::aggregate(read(&latch_n)?, latches)?
            }
        };
        latch::plug(&self.subject, bytes, latch)
    }

    /// Answer the requests sent upstream with `wasi:http` with the responses, instead of the
    /// default [`UpstreamResponse`].
    pub fn upstream(
        mut self,
        responder: impl FnMut(&UpstreamRequest) -> UpstreamResponse + Send + 'static,
    ) -> Self {
        self.responder = Box::new(responder);
        self
    }

    /// Instantiate the test subject.
    pub async fn build(self) -> Result<TestSubject> {
        let mut config = Config::new();
        config.wasm_component_model_async(true);
        // named imports, e.g. `latch0` of a `latch-n` component composed with a latch
        config.wasm_component_model_implements(true);
        let engine = Engine::new(&config)?;

        let bytes = self.compose()?;
        let component = Component::new(&engine, &bytes)
            .with_context(|| format!("failed to load {}", self.subject))?;

        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p3::add_to_linker(&mut linker)?;
        wasmtime_wasi_http::p3::add_to_linker(&mut linker)?;
        add_handler_to_linker(&mut linker)?;
        upstream_client::add_to_linker::<_, UpstreamClient>(&mut linker, |ctx| ctx)?;
        logging::add_to_linker::<_, HasSelf<Ctx>>(&mut linker, |ctx| ctx)?;
        latch::add_to_linker(&mut linker)?;

        let recorder = Recorder::default();
        let mut store = Store::new(
            &engine,
            Ctx {
                wasi: WasiCtxBuilder::new().inherit_stdio().build(),
                http: WasiHttpCtx::new(),
                upstream: Upstream {
                    recorder: recorder.clone(),
                    responder: Arc::new(Mutex::new(self.responder)),
                },
                table: ResourceTable::new(),
                recorder: recorder.clone(),
                latch: self.host_latch.unwrap_or_else(HostLatch::defer),
                config: self.config,
            },
        );
        let instance_pre = linker.instantiate_pre(&component)?;
        let instance = instance_pre
            .instantiate_async(&mut store)
            .await
            .with_context(|| format!("failed to instantiate {}", self.subject))?;
        // an interface is only available when its types match the bindings, e.g. the client
        // exported by a gate takes the host's requests, a trace component takes its own
        let exports = Exports {
            name: self.subject,
            types: types::GuestIndices::new(&instance_pre)
                .ok()
                .and_then(|indices| indices.load(&mut store, &instance).ok()),
            client: client::GuestIndices::new(&instance_pre)
                .ok()
                .and_then(|indices| indices.load(&mut store, &instance).ok()),
            handler: handler::GuestIndices::new(&instance_pre)
                .ok()
                .and_then(|indices| indices.load(&mut store, &instance).ok()),
            http_client: http_client::GuestIndices::new(&instance_pre)
                .ok()
                .and_then(|indices| indices.load(&mut store, &instance).ok()),
            gated_client: gated::client::GuestIndices::new(&instance_pre)
                .ok()
                .and_then(|indices| indices.load(&mut store, &instance).ok()),
            gated_handler: gated::handler::GuestIndices::new(&instance_pre)
                .ok()
                .and_then(|indices| indices.load(&mut store, &instance).ok()),
        };

        Ok(TestSubject {
            store,
            exports,
            recorder,
        })
    }
}

/// The test subject's exported interfaces.
///
/// Accessing an interface the component does not export panics.
pub struct Exports {
    name: String,
    types: Option<types::Guest>,
    client: Option<client::Guest>,
    handler: Option<handler::Guest>,
    http_client: Option<http_client::Guest>,
    gated_client: Option<gated::client::Guest>,
    gated_handler: Option<gated::handler::Guest>,
}

impl Exports {
    /// The exported `wasi:http/types` interface.
    pub fn wasi_http_types(&self) -> &types::Guest {
        self.types
            .as_ref()
            .unwrap_or_else(|| panic!("{} does not export wasi:http/types", self.name))
    }

    /// The exported `wasi:http/client` interface.
    pub fn wasi_http_client(&self) -> &client::Guest {
        self.client
            .as_ref()
            .unwrap_or_else(|| panic!("{} does not export wasi:http/client", self.name))
    }

    /// The exported `wasi:http/handler` interface.
    pub fn wasi_http_handler(&self) -> &handler::Guest {
        self.handler
            .as_ref()
            .unwrap_or_else(|| panic!("{} does not export wasi:http/handler", self.name))
    }

    /// The exported `componentized:http/client` interface.
    pub fn componentized_http_client(&self) -> &http_client::Guest {
        self.http_client
            .as_ref()
            .unwrap_or_else(|| panic!("{} does not export componentized:http/client", self.name))
    }

    /// The exported `wasi:http/client` of a gate, which takes requests created by the host, see
    /// [`host_request`].
    pub fn gated_client(&self) -> &gated::client::Guest {
        self.gated_client
            .as_ref()
            .unwrap_or_else(|| panic!("{} does not export wasi:http/client", self.name))
    }

    /// The exported `wasi:http/handler` of a gate, which takes requests created by the host, see
    /// [`host_request`].
    pub fn gated_handler(&self) -> &gated::handler::Guest {
        self.gated_handler
            .as_ref()
            .unwrap_or_else(|| panic!("{} does not export wasi:http/handler", self.name))
    }

    /// Whether the component exports `wasi:http/types`.
    pub fn exports_types(&self) -> bool {
        self.types.is_some()
    }

    /// Whether the component exports `wasi:http/client`.
    pub fn exports_client(&self) -> bool {
        self.client.is_some()
    }

    /// Whether the component exports `wasi:http/handler`.
    pub fn exports_handler(&self) -> bool {
        self.handler.is_some()
    }

    /// Create a request with the exported `wasi:http/types`, without a body or trailers.
    pub async fn new_request(
        &self,
        accessor: &Accessor<Ctx>,
        method: Method,
        scheme: Scheme,
        authority: &str,
        path_with_query: &str,
    ) -> Result<ResourceAny> {
        let types = self.wasi_http_types();
        let headers = types.fields().call_constructor(accessor).await?;
        let trailers = ready(accessor, Ok(None))?;
        let (request, _transmitted) = types
            .request()
            .call_new(accessor, headers, None, trailers, None)
            .await?;
        let request_methods = types.request();
        request_methods
            .call_set_method(accessor, request, method)
            .await?
            .map_err(|()| format_err!("invalid method"))?;
        request_methods
            .call_set_scheme(accessor, request, Some(scheme))
            .await?
            .map_err(|()| format_err!("invalid scheme"))?;
        request_methods
            .call_set_authority(accessor, request, Some(authority.to_string()))
            .await?
            .map_err(|()| format_err!("invalid authority"))?;
        request_methods
            .call_set_path_with_query(accessor, request, Some(path_with_query.to_string()))
            .await?
            .map_err(|()| format_err!("invalid path with query"))?;
        Ok(request)
    }
}

/// An instantiated test subject.
pub struct TestSubject {
    store: Store<Ctx>,
    exports: Exports,
    recorder: Recorder,
}

impl TestSubject {
    /// Logs and upstream requests recorded while the test subject runs.
    pub fn recorder(&self) -> Recorder {
        self.recorder.clone()
    }

    /// The test subject's exported interfaces.
    pub fn exports(&self) -> &Exports {
        &self.exports
    }

    /// Send a request through the gate's `wasi:http/client`, the status of the response or the
    /// error.
    pub async fn send(
        &mut self,
        method: http::Method,
        uri: &str,
    ) -> Result<std::result::Result<u16, HttpErrorCode>> {
        let uri = uri.to_string();
        self.run(async move |accessor, gate| {
            let request = host_request(accessor, method, &uri)?;
            Ok(
                match gate.gated_client().call_send(accessor, request).await? {
                    Ok(response) => Ok(response_status(accessor, &response)?),
                    Err(err) => Err(err),
                },
            )
        })
        .await
    }

    /// Handle a request with the gate's `wasi:http/handler`, the status of the response or the
    /// error.
    pub async fn handle(
        &mut self,
        method: http::Method,
        uri: &str,
    ) -> Result<std::result::Result<u16, HttpErrorCode>> {
        let uri = uri.to_string();
        self.run(async move |accessor, gate| {
            let request = host_request(accessor, method, &uri)?;
            Ok(
                match gate.gated_handler().call_handle(accessor, request).await? {
                    Ok(response) => Ok(response_status(accessor, &response)?),
                    Err(err) => Err(err),
                },
            )
        })
        .await
    }

    /// Run a test body against the test subject's exports.
    pub async fn run<R: Send + 'static>(
        &mut self,
        f: impl AsyncFnOnce(&Accessor<Ctx>, &Exports) -> Result<R> + Send,
    ) -> Result<R> {
        let exports = &self.exports;
        self.store
            .run_concurrent(async move |accessor| f(accessor, exports).await)
            .await?
    }
}
