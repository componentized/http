# `latch-trace`

HTTP latch that wraps another latch, logging each decision it makes without changing it.

Use it to debug a policy: wrap a latch, or an aggregate of latches, and watch the logs to see how each request is decided. Every request is authorized by the wrapped latch and its decision, or error, is returned unchanged. The log describes the request the same way the gates do.

```
Authorization DECISION=denied REASON=http-request-denied OPERATION=wasi:http/client#send METHOD=post PATH-WITH-QUERY=some</items>
```

Latch errors from the wrapped latch, for example an invalid config, are logged and returned unchanged, so the gate fails the request as it would without tracing.

```
Authorization ERROR=invalid-config<latch-method> OPERATION=wasi:http/client#send METHOD=get PATH-WITH-QUERY=some</>
```

Messages are logged at the `trace` level. For example, to trace `latch-method-readonly`:

```
package example:latch;

export new local:latch-trace {
    latch: new local:latch-method-readonly { ... }.latch,
    ...
}...;
```

## Interfaces

Imports:

- `wasi:logging/logging@0.1.0-draft`: logs the decisions of the wrapped latch
- `wasi:http/types@0.3.0`: the requests being authorized
- `componentized:http/latch@0.1.0-dev`: the wrapped latch

Exports:

- `componentized:http/latch@0.1.0-dev`: the latch
