# `client`

A higher-level HTTP client that delegates to wasi:http/client.

## Request Functions

(each returns `result<http-response, string>`)

- `request(method, url, headers, body, options)`
- `get(url, headers, options)`
- `post(url, headers, body, options)`
- `put(url, headers, body, options)`
- `delete(url, headers, options)`
- `patch(url, headers, body, options)`
- `head(url, headers, options)`
- `options(url, headers, options)`
- `trace(url, headers, options)`
- `query(url, headers, body, options)`

## Redirects

Redirects are followed, up to 10 unless `max-redirects` is set in the request options.

The request body is streamed, so it can't be sent again. A redirect that sends the body again returns a `redirect-requires-body` error with the redirect response, and the method, url, headers and options for the next request, send it with the body to follow the redirect. The body is sent again for a request with a body when the redirect keeps the method, a 307 or 308, or a 301 or 302 for a method other than POST. A 303, or a 301 or 302 for POST, switches to GET without the body, and is followed.

## The `client` World

- exports `componentized:http/client`
- imports `wasi:http/client@0.3.1`
