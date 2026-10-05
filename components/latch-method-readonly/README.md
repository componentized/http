# `latch-method-readonly`

HTTP latch that denies every request method except GET, HEAD, QUERY and OPTIONS.

`latch-method` composed with [`latch-method-readonly-config`](../latch-method-readonly-config/).

## Interfaces

Imports:

- `wasi:logging/logging@0.1.0-draft`: logs an invalid config
- `wasi:http/types@0.3.0`: the requests being authorized

Exports:

- `componentized:http/latch@0.1.0`: the latch
