# `trace-client`

Virtualizes the wasi:http/types and wasi:http/client interfaces logging all method calls at the TRACE level and 'componentized-trace' component.

Built from the sources shared with the other trace components in [`crates/trace`](../../crates/trace/src/lib.rs).

## The `trace-client` World

- exports `wasi:http/types`
- exports `wasi:http/client`
- imports `wasi:http/types`
- imports `wasi:http/client`
- imports `wasi:logging/logging`
