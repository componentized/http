# `gate-handler`

HTTP gate access control for requests handled with `wasi:http/handler`.

The latch is asked to authorize each request, then told the final decision with `observe-decision`. A denied request fails with the latch's reason and is logged as a warning:

```
Denied REASON=http-request-denied OPERATION=wasi:http/client#send METHOD=post PATH-WITH-QUERY=some</items>
```

A latch error, or a failure to observe the decision, fails the request with `internal-error` and is logged as an error:

```
Latch error CODE=invalid-config<latch-method> OPERATION=wasi:http/client#send METHOD=get PATH-WITH-QUERY=some</>
```

## Interfaces

Imports:

- `componentized:http/latch@0.1.0-dev`: decides whether each request may proceed
- `wasi:logging/logging@0.1.0-draft`: logs denials and latch errors
- `wasi:http/types@0.3.0`: the requests being gated
- `wasi:http/handler@0.3.0`: the handler the gated handler wraps

Exports:

- `wasi:http/handler@0.3.0`: handler gated by the latch
