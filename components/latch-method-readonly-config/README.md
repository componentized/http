# `latch-method-readonly-config`

Config for `latch-method` that denies every request method except GET, HEAD, QUERY and OPTIONS.

```properties
get=deferred
head=deferred
query=deferred
options=deferred
*=denied
```

## Interfaces

Exports:

- `wasi:config/store@0.2.0-rc.1`: the config
