# `authorization-injector`

Adds an `authorization` header to requests sent with `wasi:http/client` that don't already have one, with every standard method except TRACE.

`authorization-injector-core` composed with [`authorization-injector-method-config`](../authorization-injector-method-config/) for its `methods` import. The authorization for each origin is read from `wasi:config/store`, see [`authorization-injector-core`](../authorization-injector-core/).

## Interfaces

Imports:

- `wasi:config/store@0.2.0-rc.1`: the authorization for each origin
- `wasi:logging/logging@0.1.0-draft`: logs an invalid config
- `wasi:http/types@0.3.0`: the requests being sent
- `wasi:http/client@0.3.0`: the client the authorization injector wraps

Exports:

- `wasi:http/client@0.3.0`: client that adds the configured authorization
