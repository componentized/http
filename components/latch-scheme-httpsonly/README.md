# `latch-scheme-httpsonly`

HTTP latch that denies every request scheme except HTTPS.

`latch-scheme` composed with [`latch-scheme-httpsonly-config`](../latch-scheme-httpsonly-config/).

## Interfaces

Imports:

- `wasi:logging/logging@0.1.0-draft`: logs an invalid config
- `wasi:http/types@0.3.0`: the requests being authorized

Exports:

- `componentized:http/latch@0.1.0`: the latch
