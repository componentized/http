# `latch-delegate-handler`

HTTP latch that wraps another latch, delegating only `wasi:http/handler` requests to it. `wasi:http/client` requests are deferred without consulting the wrapped latch, it neither authorizes nor observes them.

Wrapping lets a latch be configured separately for outgoing and incoming requests. For example, to allow only reading methods for outgoing requests, and any method for incoming ones, wrap one `latch-method` with `latch-delegate-client` and another with `latch-delegate-handler`, give each its own config, and aggregate them with `latch-n2`:

```
package example:latch;

let client = new local:latch-method {
    store: new local:client-config {}.store,
    ...
};
let handler = new local:latch-method {
    store: new local:handler-config {}.store,
    ...
};

export new local:latch-n2 {
    latch0: new local:latch-delegate-client { latch: client.latch }.latch,
    latch1: new local:latch-delegate-handler { latch: handler.latch }.latch,
    ...
}...;
```

## Interfaces

Imports:

- `wasi:http/types@0.3.0`: the requests being authorized
- `componentized:http/latch@0.1.1-dev`: the wrapped latch, consulted for `wasi:http/handler` requests

Exports:

- `componentized:http/latch@0.1.1-dev`: the latch
