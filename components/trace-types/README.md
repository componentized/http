# `trace-types`

Virtualizes the wasi:http/types interface logging all method calls at the TRACE level and 'componentized-trace' component.

Built from the sources shared with the other trace components in [`crates/trace`](../../crates/trace/src/lib.rs).

## The `trace-types` World

- exports `wasi:http/types`
- imports `wasi:http/types`
- imports `wasi:logging/logging`
