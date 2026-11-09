# `authorization-injector-method-config`

Method config for `authorization-injector-core` that adds the authorization to requests with every standard method except TRACE. The response to a TRACE request echoes the request back, an added credential would be revealed to the caller. A method that isn't listed, e.g. a custom method, is denied.

```properties
get=deferred
head=deferred
query=deferred
options=deferred
post=deferred
put=deferred
patch=deferred
delete=deferred
connect=deferred
trace=denied
*=denied
```

## Interfaces

Exports:

- `wasi:config/store@0.2.0-rc.1`: the config
