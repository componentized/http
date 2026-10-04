# `latch-scheme-httpsonly-config`

Config for `latch-scheme` that denies every request scheme except HTTPS.

```properties
https=deferred
*=denied
```

## Interfaces

Exports:

- `wasi:config/store@0.2.0-rc.1`: the config
