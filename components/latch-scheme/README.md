# `latch-scheme`

HTTP latch that decides each request by its scheme.

The decision for each scheme is read from `wasi:config/store`. The key is the lowercase scheme, e.g. `https`, or `_` for a request without a scheme, the value is `deferred` or `denied`. A scheme without a key uses the `*` key, and is deferred without it. A denied request fails with `http-request-method-invalid`.

```properties
https=deferred
*=denied
```

Every value must be `deferred`, an empty value, or `denied`. Otherwise the cause is logged at the `critical` level and every request fails with an `invalid-config<latch-scheme>` latch error, so a typo is noticed rather than ignored:

```
Invalid config LATCH=latch-scheme KEY=post VALUE=maybe ERROR=expected one of: 'deferred', 'denied'
```

## Interfaces

Imports:

- `wasi:config/store@0.2.0-rc.1`: the decision for each scheme
- `wasi:logging/logging@0.1.0-draft`: logs an invalid config
- `wasi:http/types@0.3.0`: the requests being authorized

Exports:

- `componentized:http/latch@0.1.0`: the latch
