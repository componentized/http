# `latch-deny-all`

HTTP latch that denies every request with `http-request-denied`.

## Interfaces

Imports:

- `wasi:http/types@0.3.0`: the requests being authorized

Exports:

- `componentized:http/latch@0.1.0-dev`: the latch
