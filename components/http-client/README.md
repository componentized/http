# `http-client`

A higher-level HTTP client that delegates to `wasi:http/client`.

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

## The `http-client` World

- exports `componentized:http/client`
- imports `wasi:http/client@0.3.1`
