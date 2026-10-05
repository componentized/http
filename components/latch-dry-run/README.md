# `latch-dry-run`

HTTP latch that wraps another latch, logging the requests it would deny without denying them.

Use it to roll out a policy: wrap the latch, watch the logs for requests it would deny, and adjust the policy before enforcing it by removing the wrapper. Every request is authorized by the wrapped latch, a denial is logged as a warning and the request is deferred. The log describes the request the same way the gates do.

```
Dry run, would deny REASON=http-request-method-invalid OPERATION=wasi:http/client#send METHOD=post PATH-WITH-QUERY=some</items>
```

A latch error from the wrapped latch, for example an invalid config, is logged as an error and the request is deferred as well, a dry run never fails a request.

The wrapped latch observes the final decision with `observe-decision`, which is deferred for requests it would have denied, so a stateful latch acts on what actually happened. A failure to observe the decision is logged as an error and does not fail the request.

For example, to evaluate `latch-method-readonly`:

```
package example:latch;

export new local:latch-dry-run {
    latch: new local:latch-method-readonly { ... }.latch,
    ...
}...;
```

## Interfaces

Imports:

- `wasi:logging/logging@0.1.0-draft`: logs the requests the wrapped latch would deny, and its errors
- `wasi:http/types@0.3.0`: the requests being authorized
- `componentized:http/latch@0.1.1-dev`: the wrapped latch

Exports:

- `componentized:http/latch@0.1.1-dev`: the latch
