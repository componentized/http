use crate::{
    Trace,
    exports::wasi::http::{
        handler::Guest,
        types::{ErrorCode, Request, Response},
    },
    trace,
    types::{TraceRequest, TraceResponse},
    wasi::http::handler,
};

impl Guest for Trace {
    #[doc = " This function may be called with either an incoming request read from the"]
    #[doc = " network or a request synthesized or forwarded by another component."]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let request = request.into_inner::<TraceRequest>();
        trace!("OPERATION=wasi:http/handler#handle REQUEST={request}");
        let response = handler::handle(request.request).await?;
        Ok(Response::new(TraceResponse::new(response)))
    }
}
