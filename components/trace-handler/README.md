# `trace-handler`

Virtualizes the wasi:http/types and wasi:http/handler interfaces logging all method calls at the TRACE level and 'componentized-trace' component.

Built from the sources shared with the other trace components in [`crates/trace`](../../crates/trace/src/lib.rs).

## The `trace-handler` World

- exports `wasi:http/types`
- exports `wasi:http/handler`
- imports `wasi:http/types`
- imports `wasi:http/handler`
- imports `wasi:logging/logging`
