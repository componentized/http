use crate::{
    Trace,
    exports::wasi::http::{
        client::Guest,
        types::{ErrorCode, Request, Response},
    },
    trace,
    types::{TraceRequest, TraceResponse},
    wasi::http::client,
};

impl Guest for Trace {
    #[doc = " This function may be used to either send an outgoing request over the"]
    #[doc = " network or to forward it to another component."]
    async fn send(request: Request) -> Result<Response, ErrorCode> {
        let request = request.into_inner::<TraceRequest>();
        trace!("OPERATION=wasi:http/client#send REQUEST={request}");
        let response = client::send(request.request).await?;
        Ok(Response::new(TraceResponse::new(response)))
    }
}
