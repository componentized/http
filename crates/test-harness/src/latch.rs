//! Gates and latches: the host latch, `wasi:config/store`, and composing latch components into
//! the test subject.

use http_body_util::BodyExt;
use wac_graph::{CompositionGraph, EncodeOptions, types::Package};
use wasmtime::component::{Accessor, HasSelf, Linker, Resource};
use wasmtime::{Result, StoreContextMut, bail, format_err};

use crate::Ctx;
use crate::gate_bindings::componentized::http::latch::{
    self, ClientOperation, Decision, ErrorCode as LatchErrorCode, HandlerOperation, HttpErrorCode,
    Operation,
};
use crate::gate_bindings::wasi::config::store;

type Request = wasmtime_wasi_http::p3::Request;
type Response = wasmtime_wasi_http::p3::Response;

/// An operation the host latch was asked to authorize.
///
/// Resource handles are not retained, only the operation name and the request it is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Authorization {
    /// Operation name, e.g. `client.send`
    pub operation: String,
    /// The request method, e.g. `GET`
    pub method: String,
    /// The request path with query, e.g. `/items?page=2`
    pub path_with_query: String,
}

/// A decision the host latch was told about with `observe-decision`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    /// Operation name, e.g. `client.send`
    pub operation: String,
    /// Whether the final decision denied the request
    pub denied: bool,
}

type Policy = dyn FnMut(&Authorization) -> Result<Decision, LatchErrorCode> + Send;

/// Scripted latch implemented by the host.
///
/// It is the test subject's latch unless latch components are installed, see
/// [`crate::Harness::latch`].
pub struct HostLatch {
    policy: Box<Policy>,
    fail_observe: bool,
}

impl HostLatch {
    /// Defer every request.
    pub fn defer() -> Self {
        Self::new(|_| Ok(Decision::Deferred))
    }

    /// Deny the named operations with `http-request-denied`, defer the rest.
    pub fn deny(operations: &[&str]) -> Self {
        let operations: Vec<String> = operations.iter().map(|s| s.to_string()).collect();
        Self::new(move |auth| {
            if operations.contains(&auth.operation) {
                Ok(Decision::Denied(HttpErrorCode::HttpRequestDenied))
            } else {
                Ok(Decision::Deferred)
            }
        })
    }

    /// Fail every authorization with a latch error.
    pub fn error(message: &str) -> Self {
        let message = message.to_string();
        Self::new(move |_| Err(LatchErrorCode::Other(Some(message.clone()))))
    }

    /// Fail every authorization with an invalid config error from the named latch.
    pub fn invalid_config(latch: &str) -> Self {
        let latch = latch.to_string();
        Self::new(move |_| Err(LatchErrorCode::InvalidConfig(latch.clone())))
    }

    /// Decide each request with a custom policy.
    pub fn new(
        policy: impl FnMut(&Authorization) -> Result<Decision, LatchErrorCode> + Send + 'static,
    ) -> Self {
        Self {
            policy: Box::new(policy),
            fail_observe: false,
        }
    }

    /// Fail every `observe-decision` call.
    pub fn fail_observe(mut self) -> Self {
        self.fail_observe = true;
        self
    }
}

impl Ctx {
    fn describe(&mut self, operation: &Operation) -> Authorization {
        let (operation, request) = match operation {
            Operation::Client(ClientOperation::Send(args)) => ("client.send", &args.request),
            Operation::Handler(HandlerOperation::Handle(args)) => ("handler.handle", &args.request),
        };
        let (method, path_with_query) = match self.table.get(request) {
            Ok(request) => (
                request.method.to_string(),
                request
                    .path_with_query
                    .as_ref()
                    .map(|path| path.to_string())
                    .unwrap_or_default(),
            ),
            Err(_) => (String::new(), String::new()),
        };
        Authorization {
            operation: operation.to_string(),
            method,
            path_with_query,
        }
    }
}

impl latch::Host for Ctx {
    fn authorize(&mut self, operation: Operation) -> Result<Decision, LatchErrorCode> {
        let authorization = self.describe(&operation);
        self.recorder
            .authorizations
            .lock()
            .unwrap()
            .push(authorization.clone());
        (self.latch.policy)(&authorization)
    }

    fn observe_decision(
        &mut self,
        decision: Decision,
        operation: Operation,
    ) -> Result<(), LatchErrorCode> {
        let operation = self.describe(&operation).operation;
        self.recorder
            .observations
            .lock()
            .unwrap()
            .push(Observation {
                operation,
                denied: matches!(decision, Decision::Denied(_)),
            });
        match self.latch.fail_observe {
            true => Err(LatchErrorCode::ObservationFailed("host".to_string())),
            false => Ok(()),
        }
    }
}

impl store::Host for Ctx {
    fn get(&mut self, key: String) -> Result<Option<String>, store::Error> {
        Ok(self
            .config
            .iter()
            .rev()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.clone()))
    }

    fn get_all(&mut self) -> Result<Vec<(String, String)>, store::Error> {
        Ok(self.config.clone())
    }
}

/// Define the host latch, `wasi:config/store`, and the host latch under the named imports of a
/// `latch-n` component, e.g. `latch1`.
pub(crate) fn add_to_linker(linker: &mut Linker<Ctx>) -> Result<()> {
    latch::add_to_linker::<_, HasSelf<Ctx>>(linker, |ctx| ctx)?;
    store::add_to_linker::<_, HasSelf<Ctx>>(linker, |ctx| ctx)?;
    for slot in 0..LATCH_N_MAX {
        let mut instance = linker.instance(&format!("latch{slot}"))?;
        instance.func_wrap(
            "authorize",
            |mut store: StoreContextMut<'_, Ctx>, (operation,): (Operation,)| {
                Ok((latch::Host::authorize(store.data_mut(), operation),))
            },
        )?;
        instance.func_wrap(
            "observe-decision",
            |mut store: StoreContextMut<'_, Ctx>, (decision, operation): (Decision, Operation)| {
                Ok((latch::Host::observe_decision(
                    store.data_mut(),
                    decision,
                    operation,
                ),))
            },
        )?;
    }
    Ok(())
}

/// The most latches a `latch-n` component aggregates.
pub(crate) const LATCH_N_MAX: usize = 5;

const LATCH_INTERFACE: &str = "componentized:http/latch@0.1.0-dev";

/// Whether the latch component imports a latch, which it wraps.
pub(crate) fn imports_latch(latch: &[u8]) -> Result<bool> {
    let mut types = wac_graph::types::Types::default();
    let package = Package::from_bytes("test:latch", None, latch.to_vec(), &mut types)
        .map_err(|err| format_err!("{err:#}"))?;
    Ok(types[package.ty()].imports.contains_key(LATCH_INTERFACE))
}

/// Satisfy the leading `latch{i}` imports of a `latch-n` component with the latches, any
/// remaining slot is left for the host.
pub(crate) fn aggregate(latch_n: Vec<u8>, latches: Vec<Vec<u8>>) -> Result<Vec<u8>> {
    let mut graph = CompositionGraph::new();
    let latch_n = Package::from_bytes("test:latch-n", None, latch_n, graph.types_mut())
        .map_err(|err| format_err!("{err:#}"))?;
    let latch_n = graph.register_package(latch_n)?;
    let latch_n = graph.instantiate(latch_n);
    for (slot, latch) in latches.into_iter().enumerate() {
        let latch =
            Package::from_bytes(&format!("test:latch{slot}"), None, latch, graph.types_mut())
                .map_err(|err| format_err!("{err:#}"))?;
        let latch = graph.register_package(latch)?;
        let latch = graph.instantiate(latch);
        let export = graph.alias_instance_export(latch, LATCH_INTERFACE)?;
        graph.set_instantiation_argument(latch_n, &format!("latch{slot}"), export)?;
    }
    let export = graph.alias_instance_export(latch_n, LATCH_INTERFACE)?;
    graph.export(export, LATCH_INTERFACE)?;
    Ok(graph.encode(EncodeOptions::default())?)
}

/// Satisfy the socket's imports with the plug's exports.
pub(crate) fn plug(name: &str, socket: Vec<u8>, plug: Vec<u8>) -> Result<Vec<u8>> {
    let mut graph = CompositionGraph::new();
    let socket = Package::from_bytes("test:socket", None, socket, graph.types_mut())
        .map_err(|err| format_err!("{err:#}"))?;
    let socket = graph.register_package(socket)?;
    let plug = Package::from_bytes("test:plug", None, plug, graph.types_mut())
        .map_err(|err| format_err!("{err:#}"))?;
    let plug = graph.register_package(plug)?;
    if let Err(err) = wac_graph::plug(&mut graph, vec![plug], socket) {
        bail!("failed to plug latch into {name}: {err}");
    }
    Ok(graph.encode(EncodeOptions::default())?)
}

/// A request created by the host, for a gate's `wasi:http/client` or `wasi:http/handler`.
pub fn host_request(
    accessor: &Accessor<Ctx>,
    method: http::Method,
    uri: &str,
) -> Result<Resource<Request>> {
    let uri: http::Uri = uri.parse()?;
    let parts = uri.into_parts();
    let body = http_body_util::Empty::<bytes::Bytes>::new()
        .map_err(|never| -> wasmtime_wasi_http::Error { match never {} });
    let (request, _transmitted) = Request::new(
        method,
        parts.scheme,
        parts.authority,
        parts.path_and_query,
        wasmtime_wasi_http::FieldMap::default(),
        None,
        body,
    );
    accessor.with(|mut store| Ok(store.get().table.push(request)?))
}

/// The status of a response the host received from a gate.
pub fn response_status(accessor: &Accessor<Ctx>, response: &Resource<Response>) -> Result<u16> {
    accessor.with(|mut store| Ok(store.get().table.get(response)?.status.as_u16()))
}
