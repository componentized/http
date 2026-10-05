# `latch-deny-random`

HTTP latch that randomly denies requests, to prove a component is resilient to failures.

Every request is at risk, both requests sent with `wasi:http/client` and requests handled with `wasi:http/handler`. A denied request fails with `http-request-denied`, or the configured reason. Wrap it with [`latch-delegate-client`](../latch-delegate-client/) or [`latch-delegate-handler`](../latch-delegate-handler/) to put only some of the requests at risk.

The latch is configured with a wasi:config/store:

- `probability`: the fraction of requests denied, from `0` to `1`, defaults to `0.1`
- `seed`: an unsigned 64 bit integer, the same seed denies the same requests, defaults to a random seed from `wasi:random`
- `reason`: the wasi:http `error-code` denied requests fail with, as written in the WIT in lower case, e.g. `connection-refused` or `http-response-timeout`, defaults to `http-request-denied`. An error code with a payload has an empty payload. Any other value is reported as `internal-error` with the value as the message

The seed is logged as a warning when the latch starts, set it in the config to reproduce a failure.

```
Randomly denying wasi:http requests PROBABILITY=0.1 SEED=9383211634937261427
```

The decision for a request depends only on the seed and the number of requests observed before it. Requests another latch denies first still count, so aggregating the latch with others does not change which requests it denies.

## Interfaces

Imports:

- `wasi:logging/logging@0.1.0-draft`: logs the seed, and an invalid config
- `wasi:config/store@0.2.0-rc.1`: the latch config
- `wasi:random/insecure@0.3.0`: seeds the decisions when no `seed` is configured
- `wasi:http/types@0.3.0`: the requests being authorized

Exports:

- `componentized:http/latch@0.1.0`: the latch
