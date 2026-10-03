use std::fmt::Display;

use crate::{
    DisplayOption, Trace,
    exports::wasi::http::types::{
        Duration, ErrorCode, FieldName, FieldValue, Fields, Guest, GuestFields, GuestRequest,
        GuestRequestOptions, GuestResponse, HeaderError, Headers, Method, Request, RequestOptions,
        RequestOptionsError, Response, Scheme, StatusCode, Trailers,
    },
    fmt_handle, trace,
    wasi::http::types,
    wit_future,
};

impl Guest for Trace {
    type Fields = TraceFields;
    type Request = TraceRequest;
    type RequestOptions = TraceRequestOptions;
    type Response = TraceResponse;
}

pub(crate) struct TraceFields {
    fields: types::Fields,
}

impl TraceFields {
    fn new(fields: types::Fields) -> Self {
        Self { fields }
    }
}

impl GuestFields for TraceFields {
    #[doc = " Construct an empty HTTP Fields."]
    #[doc = ""]
    #[doc = " The resulting `fields` is mutable."]
    #[allow(async_fn_in_trait)]
    fn new() -> Self {
        trace!("OPERATION=wasi:http/types#fields.new");
        Self::new(types::Fields::new())
    }

    #[doc = " Construct an HTTP Fields."]
    #[doc = ""]
    #[doc = " The resulting `fields` is mutable."]
    #[doc = ""]
    #[doc = " The list represents each name-value pair in the Fields. Names"]
    #[doc = " which have multiple values are represented by multiple entries in this"]
    #[doc = " list with the same name."]
    #[doc = ""]
    #[doc = " The tuple is a pair of the field name, represented as a string, and"]
    #[doc = " Value, represented as a list of bytes. In a valid Fields, all names"]
    #[doc = " and values are valid UTF-8 strings. However, values are not always"]
    #[doc = " well-formed, so they are represented as a raw list of bytes."]
    #[doc = ""]
    #[doc = " An error result will be returned if any header or value was"]
    #[doc = " syntactically invalid, if a header was forbidden, or if the"]
    #[doc = " entries would exceed an implementation size limit."]
    #[allow(async_fn_in_trait)]
    fn from_list(entries: Vec<(FieldName, FieldValue)>) -> Result<Fields, HeaderError> {
        trace!(
            "OPERATION=wasi:http/types#fields.from-list ENTRIES-LEN={}",
            entries.len()
        );
        match types::Fields::from_list(&entries) {
            Ok(fields) => Ok(Fields::new(Self::new(fields))),
            Err(err) => Err(err),
        }
    }

    #[doc = " Get all of the values corresponding to a name. If the name is not present"]
    #[doc = " in this `fields`, an empty list is returned. However, if the name is"]
    #[doc = " present but empty, this is represented by a list with one or more"]
    #[doc = " empty field-values present."]
    #[allow(async_fn_in_trait)]
    fn get(&self, name: FieldName) -> Vec<FieldValue> {
        trace!("OPERATION=wasi:http/types#fields.get SELF={self} NAME={name}");
        self.fields.get(&name)
    }

    #[doc = " Returns `true` when the name is present in this `fields`. If the name is"]
    #[doc = " syntactically invalid, `false` is returned."]
    #[allow(async_fn_in_trait)]
    fn has(&self, name: FieldName) -> bool {
        trace!("OPERATION=wasi:http/types#fields.has SELF={self} NAME={name}");
        self.fields.has(&name)
    }

    #[doc = " Set all of the values for a name. Clears any existing values for that"]
    #[doc = " name, if they have been set."]
    #[doc = ""]
    #[doc = " Fails with `header-error.immutable` if the `fields` are immutable."]
    #[doc = ""]
    #[doc = " Fails with `header-error.size-exceeded` if the name or values would"]
    #[doc = " exceed an implementation-defined size limit."]
    #[allow(async_fn_in_trait)]
    fn set(&self, name: FieldName, value: Vec<FieldValue>) -> Result<(), HeaderError> {
        trace!(
            "OPERATION=wasi:http/types#fields.set SELF={self} NAME={name} VALUE-LEN={}",
            value.len()
        );
        self.fields.set(&name, &value)
    }

    #[doc = " Delete all values for a name. Does nothing if no values for the name"]
    #[doc = " exist."]
    #[doc = ""]
    #[doc = " Fails with `header-error.immutable` if the `fields` are immutable."]
    #[allow(async_fn_in_trait)]
    fn delete(&self, name: FieldName) -> Result<(), HeaderError> {
        trace!("OPERATION=wasi:http/types#fields.delete SELF={self} NAME={name}");
        self.fields.delete(&name)
    }

    #[doc = " Delete all values for a name. Does nothing if no values for the name"]
    #[doc = " exist."]
    #[doc = ""]
    #[doc = " Returns all values previously corresponding to the name, if any."]
    #[doc = ""]
    #[doc = " Fails with `header-error.immutable` if the `fields` are immutable."]
    #[allow(async_fn_in_trait)]
    fn get_and_delete(&self, name: FieldName) -> Result<Vec<FieldValue>, HeaderError> {
        trace!("OPERATION=wasi:http/types#fields.get-and-delete SELF={self} NAME={name}");
        self.fields.get_and_delete(&name)
    }

    #[doc = " Append a value for a name. Does not change or delete any existing"]
    #[doc = " values for that name."]
    #[doc = ""]
    #[doc = " Fails with `header-error.immutable` if the `fields` are immutable."]
    #[doc = ""]
    #[doc = " Fails with `header-error.size-exceeded` if the value would exceed"]
    #[doc = " an implementation-defined size limit."]
    #[allow(async_fn_in_trait)]
    fn append(&self, name: FieldName, value: FieldValue) -> Result<(), HeaderError> {
        trace!(
            "OPERATION=wasi:http/types#fields.append SELF={self} NAME={name} VALUE={value}",
            value = DisplayFieldValue(value.clone())
        );
        self.fields.append(&name, &value)
    }

    #[doc = " Retrieve the full set of names and values in the Fields. Like the"]
    #[doc = " constructor, the list represents each name-value pair."]
    #[doc = ""]
    #[doc = " The outer list represents each name-value pair in the Fields. Names"]
    #[doc = " which have multiple values are represented by multiple entries in this"]
    #[doc = " list with the same name."]
    #[doc = ""]
    #[doc = " The names and values are always returned in the original casing and in"]
    #[doc = " the order in which they will be serialized for transport."]
    #[allow(async_fn_in_trait)]
    fn copy_all(&self) -> Vec<(FieldName, FieldValue)> {
        trace!("OPERATION=wasi:http/types#fields.copy-all SELF={self}");
        self.fields.copy_all()
    }

    #[doc = " Make a deep copy of the Fields. Equivalent in behavior to calling the"]
    #[doc = " `fields` constructor on the return value of `copy-all`. The resulting"]
    #[doc = " `fields` is mutable."]
    #[allow(async_fn_in_trait)]
    fn clone(&self) -> Fields {
        trace!("OPERATION=wasi:http/types#fields.clone SELF={self}");
        Fields::new(Self::new(self.fields.clone()))
    }
}

impl Display for TraceFields {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt_handle(f, self.fields.handle())
    }
}

struct DisplayFieldValue(FieldValue);

impl Display for DisplayFieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(value) = self;
        f.write_str(&String::from_utf8_lossy(value))
    }
}

pub(crate) struct TraceRequest {
    pub(crate) request: types::Request,
}

impl TraceRequest {
    pub(crate) fn new(request: types::Request) -> Self {
        Self { request }
    }
}

impl GuestRequest for TraceRequest {
    #[doc = " Construct a new `request` with a default `method` of `GET`, and"]
    #[doc = " `none` values for `path-with-query`, `scheme`, and `authority`."]
    #[doc = ""]
    #[doc = " `headers` is the HTTP Headers for the Request."]
    #[doc = ""]
    #[doc = " `contents` is the optional body content stream with `none`"]
    #[doc = " representing a zero-length content stream."]
    #[doc = " Once it is closed, `trailers` future must resolve to a result."]
    #[doc = " If `trailers` resolves to an error, underlying connection"]
    #[doc = " will be closed immediately."]
    #[doc = ""]
    #[doc = " `options` is optional `request-options` resource to be used"]
    #[doc = " if the request is sent over a network connection."]
    #[doc = ""]
    #[doc = " It is possible to construct, or manipulate with the accessor functions"]
    #[doc = " below, a `request` with an invalid combination of `scheme`"]
    #[doc = " and `authority`, or `headers` which are not permitted to be sent."]
    #[doc = " It is the obligation of the `handler.handle` implementation"]
    #[doc = " to reject invalid constructions of `request`."]
    #[doc = ""]
    #[doc = " The returned future resolves to result of transmission of this request."]
    #[allow(async_fn_in_trait)]
    fn new(
        headers: Headers,
        contents: Option<wit_bindgen::StreamReader<u8>>,
        trailers: wit_bindgen::FutureReader<Result<Option<Trailers>, ErrorCode>>,
        options: Option<RequestOptions>,
    ) -> (Request, wit_bindgen::FutureReader<Result<(), ErrorCode>>) {
        trace!("OPERATION=wasi:http/types#request.new");
        let headers = headers.into_inner::<TraceFields>().fields;
        let (trailers_tx, trailers_rx) = wit_future::new(|| Ok(None));
        wit_bindgen::spawn_local(async move {
            let trailers = match trailers.await {
                Ok(None) => Ok(None),
                Ok(Some(trailers)) => Ok(Some(trailers.into_inner::<TraceFields>().fields)),
                Err(err) => Err(err),
            };
            let _ = trailers_tx.write(trailers).await;
        });
        let options = options.map(|options| options.into_inner::<TraceRequestOptions>().options);
        let (request, read) = types::Request::new(headers, contents, trailers_rx, options);
        (Request::new(Self::new(request)), read)
    }

    #[doc = " Get the Method for the Request."]
    #[allow(async_fn_in_trait)]
    fn get_method(&self) -> Method {
        trace!("OPERATION=wasi:http/types#request.get-method SELF={self}");
        self.request.get_method()
    }

    #[doc = " Set the Method for the Request. Fails if the string present in a"]
    #[doc = " `method.other` argument is not a syntactically valid method."]
    #[allow(async_fn_in_trait)]
    fn set_method(&self, method: Method) -> Result<(), ()> {
        trace!("OPERATION=wasi:http/types#request.set-method SELF={self} METHOD={method}");
        self.request.set_method(&method)
    }

    #[doc = " Get the combination of the HTTP Path and Query for the Request.  When"]
    #[doc = " `none`, this represents an empty Path and empty Query."]
    #[allow(async_fn_in_trait)]
    fn get_path_with_query(&self) -> Option<String> {
        trace!("OPERATION=wasi:http/types#request.get-path-with-query SELF={self}");
        self.request.get_path_with_query()
    }

    #[doc = " Set the combination of the HTTP Path and Query for the Request.  When"]
    #[doc = " `none`, this represents an empty Path and empty Query. Fails is the"]
    #[doc = " string given is not a syntactically valid path and query uri component."]
    #[allow(async_fn_in_trait)]
    fn set_path_with_query(&self, path_with_query: Option<String>) -> Result<(), ()> {
        trace!(
            "OPERATION=wasi:http/types#request.set-path-with-query SELF={self} PATH-WITH-QUERY={path_with_query}",
            path_with_query = DisplayOption(path_with_query.clone())
        );
        self.request.set_path_with_query(path_with_query.as_deref())
    }

    #[doc = " Get the HTTP Related Scheme for the Request. When `none`, the"]
    #[doc = " implementation may choose an appropriate default scheme."]
    #[allow(async_fn_in_trait)]
    fn get_scheme(&self) -> Option<Scheme> {
        trace!("OPERATION=wasi:http/types#request.get-scheme SELF={self}");
        self.request.get_scheme()
    }

    #[doc = " Set the HTTP Related Scheme for the Request. When `none`, the"]
    #[doc = " implementation may choose an appropriate default scheme. Fails if the"]
    #[doc = " string given is not a syntactically valid uri scheme."]
    #[allow(async_fn_in_trait)]
    fn set_scheme(&self, scheme: Option<Scheme>) -> Result<(), ()> {
        trace!(
            "OPERATION=wasi:http/types#request.set-scheme SELF={self} SCHEME={scheme}",
            scheme = DisplayOption(scheme.clone())
        );
        self.request.set_scheme(scheme.as_ref())
    }

    #[doc = " Get the authority of the Request\'s target URI. A value of `none` may be used"]
    #[doc = " with Related Schemes which do not require an authority. The HTTP and"]
    #[doc = " HTTPS schemes always require an authority."]
    #[allow(async_fn_in_trait)]
    fn get_authority(&self) -> Option<String> {
        trace!("OPERATION=wasi:http/types#request.get-authority SELF={self}");
        self.request.get_authority()
    }

    #[doc = " Set the authority of the Request\'s target URI. A value of `none` may be used"]
    #[doc = " with Related Schemes which do not require an authority. The HTTP and"]
    #[doc = " HTTPS schemes always require an authority. Fails if the string given is"]
    #[doc = " not a syntactically valid URI authority."]
    #[allow(async_fn_in_trait)]
    fn set_authority(&self, authority: Option<String>) -> Result<(), ()> {
        trace!(
            "OPERATION=wasi:http/types#request.set-authority SELF={self} AUTHORITY={authority}",
            authority = DisplayOption(authority.clone())
        );
        self.request.set_authority(authority.as_deref())
    }

    #[doc = " Get the `request-options` to be associated with this request"]
    #[doc = ""]
    #[doc = " The returned `request-options` resource is immutable: `set-*` operations"]
    #[doc = " will fail if invoked."]
    #[doc = ""]
    #[doc = " This `request-options` resource is a child: it must be dropped before"]
    #[doc = " the parent `request` is dropped, or its ownership is transferred to"]
    #[doc = " another component by e.g. `handler.handle`."]
    #[allow(async_fn_in_trait)]
    fn get_options(&self) -> Option<RequestOptions> {
        trace!("OPERATION=wasi:http/types#request.get-options SELF={self}");
        self.request
            .get_options()
            .map(|options| RequestOptions::new(TraceRequestOptions::new(options)))
    }

    #[doc = " Get the headers associated with the Request."]
    #[doc = ""]
    #[doc = " The returned `headers` resource is immutable: `set`, `append`, and"]
    #[doc = " `delete` operations will fail with `header-error.immutable`."]
    #[allow(async_fn_in_trait)]
    fn get_headers(&self) -> Headers {
        trace!("OPERATION=wasi:http/types#request.get-headers SELF={self}");
        Headers::new(TraceFields::new(self.request.get_headers()))
    }

    #[doc = " Get body of the Request."]
    #[doc = ""]
    #[doc = " Stream returned by this method represents the contents of the body."]
    #[doc = " Once the stream is reported as closed, callers should await the returned"]
    #[doc = " future to determine whether the body was received successfully."]
    #[doc = " The future will only resolve after the stream is reported as closed."]
    #[doc = ""]
    #[doc = " This function takes a `res` future as a parameter, which can be used to"]
    #[doc = " communicate an error in handling of the request."]
    #[doc = ""]
    #[doc = " Note that function will move the `request`, but references to headers or"]
    #[doc = " request options acquired from it previously will remain valid."]
    #[allow(async_fn_in_trait)]
    fn consume_body(
        this: Request,
        res: wit_bindgen::FutureReader<Result<(), ErrorCode>>,
    ) -> (
        wit_bindgen::StreamReader<u8>,
        wit_bindgen::FutureReader<Result<Option<Trailers>, ErrorCode>>,
    ) {
        let this = this.into_inner::<TraceRequest>();
        trace!("OPERATION=wasi:http/types#request.consume-body THIS={this}");
        let request = this.request;
        let (contents, trailers) = types::Request::consume_body(request, res);
        let (trailers_tx, trailers_rx) = wit_future::new(|| Ok(None));
        wit_bindgen::spawn_local(async move {
            let trailers = match trailers.await {
                Ok(None) => Ok(None),
                Ok(Some(trailers)) => Ok(Some(Trailers::new(TraceFields::new(trailers)))),
                Err(err) => Err(err),
            };
            let _ = trailers_tx.write(trailers).await;
        });
        (contents, trailers_rx)
    }
}

impl Display for TraceRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt_handle(f, self.request.handle())
    }
}

impl Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            types::Method::Get => f.write_str("get"),
            types::Method::Head => f.write_str("head"),
            types::Method::Post => f.write_str("post"),
            types::Method::Put => f.write_str("put"),
            types::Method::Delete => f.write_str("delete"),
            types::Method::Connect => f.write_str("connect"),
            types::Method::Options => f.write_str("options"),
            types::Method::Trace => f.write_str("trace"),
            types::Method::Patch => f.write_str("patch"),
            types::Method::Other(method) => f.write_str(&method.to_ascii_lowercase()),
        }
    }
}

impl Display for Scheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            types::Scheme::Http => f.write_str("http"),
            types::Scheme::Https => f.write_str("https"),
            types::Scheme::Other(scheme) => write!(f, "other<{scheme}>"),
        }
    }
}

pub(crate) struct TraceRequestOptions {
    options: types::RequestOptions,
}

impl TraceRequestOptions {
    fn new(options: types::RequestOptions) -> Self {
        Self { options }
    }
}

impl GuestRequestOptions for TraceRequestOptions {
    #[doc = " Construct a default `request-options` value."]
    #[allow(async_fn_in_trait)]
    fn new() -> Self {
        trace!("OPERATION=wasi:http/types#request-options.new");
        Self::new(types::RequestOptions::new())
    }

    #[doc = " The timeout for the initial connect to the HTTP Server."]
    #[allow(async_fn_in_trait)]
    fn get_connect_timeout(&self) -> Option<Duration> {
        trace!("OPERATION=wasi:http/types#request-options.get-connect-timeout SELF={self}");
        self.options.get_connect_timeout()
    }

    #[doc = " Set the timeout for the initial connect to the HTTP Server. An error"]
    #[doc = " return value indicates that this timeout is not supported or that this"]
    #[doc = " handle is immutable."]
    #[allow(async_fn_in_trait)]
    fn set_connect_timeout(&self, duration: Option<Duration>) -> Result<(), RequestOptionsError> {
        trace!(
            "OPERATION=wasi:http/types#request-options.set-connect-timeout SELF={self} DURATION={duration}",
            duration = DisplayOption(duration)
        );
        self.options.set_connect_timeout(duration)
    }

    #[doc = " The timeout for receiving the first byte of the Response body."]
    #[allow(async_fn_in_trait)]
    fn get_first_byte_timeout(&self) -> Option<Duration> {
        trace!("OPERATION=wasi:http/types#request-options.get-first-byte-timeout SELF={self}");
        self.options.get_first_byte_timeout()
    }

    #[doc = " Set the timeout for receiving the first byte of the Response body. An"]
    #[doc = " error return value indicates that this timeout is not supported or that"]
    #[doc = " this handle is immutable."]
    #[allow(async_fn_in_trait)]
    fn set_first_byte_timeout(
        &self,
        duration: Option<Duration>,
    ) -> Result<(), RequestOptionsError> {
        trace!(
            "OPERATION=wasi:http/types#request-options.set-first-byte-timeout SELF={self} DURATION={duration}",
            duration = DisplayOption(duration)
        );
        self.options.set_first_byte_timeout(duration)
    }

    #[doc = " The timeout for receiving subsequent chunks of bytes in the Response"]
    #[doc = " body stream."]
    #[allow(async_fn_in_trait)]
    fn get_between_bytes_timeout(&self) -> Option<Duration> {
        trace!("OPERATION=wasi:http/types#request-options.get-between-bytes-timeout SELF={self}");
        self.options.get_between_bytes_timeout()
    }

    #[doc = " Set the timeout for receiving subsequent chunks of bytes in the Response"]
    #[doc = " body stream. An error return value indicates that this timeout is not"]
    #[doc = " supported or that this handle is immutable."]
    #[allow(async_fn_in_trait)]
    fn set_between_bytes_timeout(
        &self,
        duration: Option<Duration>,
    ) -> Result<(), RequestOptionsError> {
        trace!(
            "OPERATION=wasi:http/types#request-options.set-between-bytes-timeout SELF={self} DURATION={duration}",
            duration = DisplayOption(duration)
        );
        self.options.set_between_bytes_timeout(duration)
    }

    #[doc = " Make a deep copy of the `request-options`."]
    #[doc = " The resulting `request-options` is mutable."]
    #[allow(async_fn_in_trait)]
    fn clone(&self) -> RequestOptions {
        trace!("OPERATION=wasi:http/types#request-options.clone SELF={self}");
        RequestOptions::new(Self::new(self.options.clone()))
    }
}

impl Display for TraceRequestOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt_handle(f, self.options.handle())
    }
}

pub(crate) struct TraceResponse {
    response: types::Response,
}

impl TraceResponse {
    pub(crate) fn new(response: types::Response) -> Self {
        Self { response }
    }
}

impl GuestResponse for TraceResponse {
    #[doc = " Construct a new `response`, with a default `status-code` of `200`."]
    #[doc = " If a different `status-code` is needed, it must be set via the"]
    #[doc = " `set-status-code` method."]
    #[doc = ""]
    #[doc = " `headers` is the HTTP Headers for the Response."]
    #[doc = ""]
    #[doc = " `contents` is the optional body content stream with `none`"]
    #[doc = " representing a zero-length content stream."]
    #[doc = " Once it is closed, `trailers` future must resolve to a result."]
    #[doc = " If `trailers` resolves to an error, underlying connection"]
    #[doc = " will be closed immediately."]
    #[doc = ""]
    #[doc = " The returned future resolves to result of transmission of this response."]
    #[allow(async_fn_in_trait)]
    fn new(
        headers: Headers,
        contents: Option<wit_bindgen::StreamReader<u8>>,
        trailers: wit_bindgen::FutureReader<Result<Option<Trailers>, ErrorCode>>,
    ) -> (Response, wit_bindgen::FutureReader<Result<(), ErrorCode>>) {
        trace!("OPERATION=wasi:http/types#response.new");
        let headers = headers.into_inner::<TraceFields>().fields;
        let (trailers_tx, trailers_rx) = wit_future::new(|| Ok(None));
        wit_bindgen::spawn_local(async move {
            let trailers = match trailers.await {
                Ok(None) => Ok(None),
                Ok(Some(trailers)) => Ok(Some(trailers.into_inner::<TraceFields>().fields)),
                Err(err) => Err(err),
            };
            let _ = trailers_tx.write(trailers).await;
        });
        let (response, read) = types::Response::new(headers, contents, trailers_rx);
        (Response::new(Self::new(response)), read)
    }

    #[doc = " Get the HTTP Status Code for the Response."]
    #[allow(async_fn_in_trait)]
    fn get_status_code(&self) -> StatusCode {
        trace!("OPERATION=wasi:http/types#response.get-status-code SELF={self}");
        self.response.get_status_code()
    }

    #[doc = " Set the HTTP Status Code for the Response. Fails if the status-code"]
    #[doc = " given is not a valid http status code."]
    #[allow(async_fn_in_trait)]
    fn set_status_code(&self, status_code: StatusCode) -> Result<(), ()> {
        trace!(
            "OPERATION=wasi:http/types#response.set-status-code SELF={self} STATUS-CODE={status_code}"
        );
        self.response.set_status_code(status_code)
    }

    #[doc = " Get the headers associated with the Response."]
    #[doc = ""]
    #[doc = " The returned `headers` resource is immutable: `set`, `append`, and"]
    #[doc = " `delete` operations will fail with `header-error.immutable`."]
    #[allow(async_fn_in_trait)]
    fn get_headers(&self) -> Headers {
        trace!("OPERATION=wasi:http/types#response.get-headers SELF={self}");
        Headers::new(TraceFields::new(self.response.get_headers()))
    }

    #[doc = " Get body of the Response."]
    #[doc = ""]
    #[doc = " Stream returned by this method represents the contents of the body."]
    #[doc = " Once the stream is reported as closed, callers should await the returned"]
    #[doc = " future to determine whether the body was received successfully."]
    #[doc = " The future will only resolve after the stream is reported as closed."]
    #[doc = ""]
    #[doc = " This function takes a `res` future as a parameter, which can be used to"]
    #[doc = " communicate an error in handling of the response."]
    #[doc = ""]
    #[doc = " Note that function will move the `response`, but references to headers"]
    #[doc = " acquired from it previously will remain valid."]
    #[allow(async_fn_in_trait)]
    fn consume_body(
        this: Response,
        res: wit_bindgen::FutureReader<Result<(), ErrorCode>>,
    ) -> (
        wit_bindgen::StreamReader<u8>,
        wit_bindgen::FutureReader<Result<Option<Trailers>, ErrorCode>>,
    ) {
        let this = this.into_inner::<TraceResponse>();
        trace!("OPERATION=wasi:http/types#response.consume-body THIS={this}");
        let response = this.response;
        let (contents, trailers) = types::Response::consume_body(response, res);
        let (trailers_tx, trailers_rx) = wit_future::new(|| Ok(None));
        wit_bindgen::spawn_local(async move {
            let trailers = match trailers.await {
                Ok(None) => Ok(None),
                Ok(Some(trailers)) => Ok(Some(Trailers::new(TraceFields::new(trailers)))),
                Err(err) => Err(err),
            };
            let _ = trailers_tx.write(trailers).await;
        });
        (contents, trailers_rx)
    }
}

impl Display for TraceResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt_handle(f, self.response.handle())
    }
}
