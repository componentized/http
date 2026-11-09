# `authorization-injector-core`

Adds an `authorization` header to requests sent with `wasi:http/client` that don't already have one, by the request's origin and method.

A request that already has an `authorization` header, whose method isn't allowed, or whose origin no key matches, is sent unchanged. [`authorization-injector`](../authorization-injector/) is this component with a method config that allows every standard method except TRACE.

## Credentials

The value for each origin is read from `wasi:config/store`. The key is the origin, `scheme://host[:port]`, and the value is the `authorization` header to send.

```properties
https://api.example.com=Bearer 0123456789abcdef
https://*.example.com=Basic dXNlcjpwYXNz
http://localhost:*=Bearer dev-token
https://public.example.com=
```

- Without a port, a key matches the scheme's default port, e.g. `https://example.com` matches `https://example.com:443` but not `https://example.com:8443`.
- A host starting with `*.` matches the domain and every subdomain, e.g. `*.example.com` matches `example.com`, `foo.example.com` and `a.b.example.com`, but not `notexample.com`. `*` is only allowed as the first label of the host.
- A port of `*` matches any port, including the default.
- Schemes and hosts match case-insensitively. Userinfo in the request authority is ignored.
- When several keys match, the most specific one is used: an exact host before a wildcard host, a longer wildcard domain before a shorter one, then an exact port before `*`.
- An empty value sends no `authorization` header, e.g. to exclude `public.example.com` from `*.example.com` above.

## Methods

Whether the authorization is added to a request with each method is read from the `methods` import, a `wasi:config/store`. The key is the lowercase method, e.g. `get` or `post`, the value is `deferred` or `denied`. A method without a key uses the `*` key, and is denied without it, so without a method config no authorization is added.

```properties
get=deferred
post=deferred
trace=denied
*=denied
```

Deny `trace`: the response to a TRACE request echoes the request back, an added header would reveal the credential to the caller.

## Invalid config

Every credential key must be a valid origin, no two keys may match the same origins, e.g. `https://example.com` and `https://example.com:443`, and every credential must be a valid header value. Every method value must be `deferred`, an empty value, or `denied`. Otherwise the cause is logged at the `critical` level and every request fails with `internal-error`, so a typo is noticed rather than ignored. A credential is never logged:

```
Invalid config COMPONENT=authorization-injector-core CONFIG=credentials KEY=api.example.com ERROR=expected 'scheme://host[:port]'
Invalid config COMPONENT=authorization-injector-core CONFIG=methods KEY=post VALUE=allowed ERROR=expected one of: 'deferred', 'denied'
```

> [!CAUTION]
> The configured values are credentials. Any request the wrapped component sends to a matching origin with an allowed method carries the credential, whatever its path. Configure the narrowest keys and methods you can, and prefer `https`.

The headers of a `wasi:http` request are immutable, so a request that gets an `authorization` header is sent as a new request, with a copy of its headers, the same method, target and options, and its body and trailers. The result of transmitting the new request is forwarded to the original request.

## Interfaces

Imports:

- `wasi:config/store@0.2.0-rc.1`: the authorization for each origin
- `methods: wasi:config/store@0.2.0-rc.1`: whether the authorization is added for each method
- `wasi:logging/logging@0.1.0-draft`: logs an invalid config
- `wasi:http/types@0.3.0`: the requests being sent
- `wasi:http/client@0.3.0`: the client the authorization injector wraps

Exports:

- `wasi:http/client@0.3.0`: client that adds the configured authorization
