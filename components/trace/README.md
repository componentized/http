# `trace`

Virtualizes the wasi:http/types, wasi:http/client and wasi:http/handler interfaces logging all method calls at the TRACE level and 'componentized-trace' component.

Built from the sources shared with the other trace components in [`crates/trace`](../../crates/trace/src/lib.rs).

## The `trace` World

- exports `wasi:http/types`
- exports `wasi:http/client`
- exports `wasi:http/handler`
- imports `wasi:http/types`
- imports `wasi:http/client`
- imports `wasi:http/handler`
- imports `wasi:logging/logging`
