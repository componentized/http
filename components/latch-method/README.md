# `latch-method`

HTTP latch that decides each request by its method.

The decision for each method is read from `wasi:config/store`. The key is the lowercase method, e.g. `get` or `post`, the value is `deferred` or `denied`. A method without a key uses the `*` key, and is deferred without it. A denied request fails with `http-request-method-invalid`.

```properties
get=deferred
head=deferred
*=denied
```

Every value must be `deferred`, an empty value, or `denied`. Otherwise the cause is logged at the `critical` level and every request fails with an `invalid-config<latch-method>` latch error, so a typo is noticed rather than ignored:

```
Invalid config LATCH=latch-method KEY=post VALUE=maybe ERROR=expected one of: 'deferred', 'denied'
```

## Interfaces

Imports:

- `wasi:config/store@0.2.0-rc.1`: the decision for each method
- `wasi:logging/logging@0.1.0-draft`: logs an invalid config
- `wasi:http/types@0.3.0`: the requests being authorized

Exports:

- `componentized:http/latch@0.1.0`: the latch
