# HTTP Components <!-- omit in toc -->

A collection of utility components that remix wasi:http types and interfaces.

- [Components](#components)
  - [Gates](#gates)
  - [Latches](#latches)
    - [Blanket decisions](#blanket-decisions)
    - [Request properties](#request-properties)
    - [Fault injection](#fault-injection)
    - [Combining latches](#combining-latches)
  - [Tracing](#tracing)
  - [Choosing a component variant](#choosing-a-component-variant)
- [Build](#build)
- [Community](#community)
  - [Code of Conduct](#code-of-conduct)
  - [Communication](#communication)
  - [Contributing](#contributing)
- [Acknowledgements](#acknowledgements)
- [License](#license)


## Components

- [`client`](./components/client/): a higher-level HTTP client that delegates to `wasi:http/client`
- [`status-codes`](./components/status-codes/): constants for HTTP status codes

### Gates

Access control for wasi:http is split between gates and latches. A gate wraps `wasi:http/client` or `wasi:http/handler` and consults a latch before each request, a latch decides whether the request may proceed. Latches are small and single purpose, combine them to build a policy.

- [`gate`](./components/gate/): gates both `wasi:http/client` and `wasi:http/handler`
- [`gate-client`](./components/gate-client/): gates `wasi:http/client`, requests sent
- [`gate-handler`](./components/gate-handler/): gates `wasi:http/handler`, requests handled

A denied request fails with the latch's reason, and is logged as a warning. A latch error fails the request with `internal-error`, and is logged as an error.

> [!CAUTION]
> Interfering with HTTP requests can have dramatic, unintended consequences. A denied request surfaces to the caller as a failed request, which can trigger retries, timeouts and fallbacks far from the request that was denied. Install new latches, and new configurations of existing latches, cautiously and monitor the result: roll out with [`latch-dry-run`](./components/latch-dry-run/), watch decisions with [`latch-trace`](./components/latch-trace/), and review the denials the gate logs.

### Latches

Decide which requests are allowed. A latch defers or denies, a request proceeds unless a latch denies it. Deciding and acting on a decision are separate steps, a latch is told the final decision for each request with `observe-decision`.

#### Blanket decisions

- [`latch-defer-all`](./components/latch-defer-all/): defers every request, allowing all requests
- [`latch-deny-all`](./components/latch-deny-all/): denies every request

#### Request properties

- [`latch-method`](./components/latch-method/): decides by the request method, configured with `wasi:config/store`
- [`latch-method-readonly`](./components/latch-method-readonly/): allows only GET, HEAD, QUERY and OPTIONS, `latch-method` with [`latch-method-readonly-config`](./components/latch-method-readonly-config/)
- [`latch-scheme`](./components/latch-scheme/): decides by the request scheme, configured with `wasi:config/store`
- [`latch-scheme-httpsonly`](./components/latch-scheme-httpsonly/): allows only HTTPS, `latch-scheme` with [`latch-scheme-httpsonly-config`](./components/latch-scheme-httpsonly-config/)

#### Fault injection

Deny requests on purpose, to prove a component is resilient to failures.

- [`latch-deny-random`](./components/latch-deny-random/): randomly denies a configurable fraction of requests, reproducible with a seed

#### Combining latches

Build a policy from several latches, apply a latch to only part of the requests, or try a policy before enforcing it.

- [`latch-n2`](./components/latch-n2/), [`latch-n3`](./components/latch-n3/), [`latch-n4`](./components/latch-n4/), [`latch-n5`](./components/latch-n5/): aggregate two to five latches, any latch can deny a request
- [`latch-delegate-client`](./components/latch-delegate-client/) / [`latch-delegate-handler`](./components/latch-delegate-handler/): apply a wrapped latch to only `wasi:http/client` or only `wasi:http/handler` requests
- [`latch-dry-run`](./components/latch-dry-run/): log what a wrapped latch would deny without enforcing it, to roll out a policy
- [`latch-trace`](./components/latch-trace/): log the decisions of a wrapped latch

### Tracing

Log wasi:http calls, for debugging or auditing, without affecting them.

- [`trace`](./components/trace/): traces `wasi:http/types`, `wasi:http/client` and `wasi:http/handler`
- [`trace-client`](./components/trace-client/): traces `wasi:http/types` and `wasi:http/client`
- [`trace-handler`](./components/trace-handler/): traces `wasi:http/types` and `wasi:http/handler`
- [`trace-types`](./components/trace-types/): traces `wasi:http/types`
- [`trace-componentized-client`](./components/trace-componentized-client/): traces `componentized:http/client`

### Choosing a component variant

Due to resource types being unique to the instance that defines them in the Component Model, fine grain composition of the `wasi:http` interfaces can be persnickety. Pick the most specific component that covers the interfaces the target component imports. While the base component is more universal, a larger surface area asks the host for capabilities the target component doesn't use.

For example, with the `trace-*` components:

| The target component imports                                  | Use                                                                      |
| ------------------------------------------------------------- | ------------------------------------------------------------------------ |
| `wasi:http/types`                                             | [`trace-types`](./components/trace-types/)                               |
| `wasi:http/types` and `wasi:http/client`                      | [`trace-client`](./components/trace-client/)                             |
| `wasi:http/types` and `wasi:http/handler`                     | [`trace-handler`](./components/trace-handler/)                           |
| `wasi:http/types`, `wasi:http/client` and `wasi:http/handler` | [`trace`](./components/trace/)                                           |
| `componentized:http/client`                                   | [`trace-componentized-client`](./components/trace-componentized-client/) |

`trace-client`, `trace-handler` and `trace` also trace `wasi:http/types`, don't combine them with `trace-types`.

The `wasi:http/handler` exported by `trace-handler` and `trace` takes requests created with their exported `wasi:http/types`. Use them in front of a component that forwards requests with `wasi:http/handler`, a host serving incoming requests can't call them directly.

## Build

Prereqs:
- a rust toolchain
- [`cargo-binstall`](https://github.com/cargo-bins/cargo-binstall), optional, to download prebuilt tools instead of building them

```sh
make components
```

The build creates each component in [`components`](./components) into `target/components`, e.g. the client at `target/components/client/client.wasm`, along with `target/components/interface.wasm`, the `componentized:http` WIT package. Each component is also built with debug info, e.g. `target/components/client/client.debug.wasm`.

The cli tools the build uses, [`static-config`](https://github.com/componentized/static-config), [`wasm-tools`](https://github.com/bytecodealliance/wasm-tools), [`wac`](https://github.com/bytecodealliance/wac) and [`wkg`](https://github.com/bytecodealliance/wasm-pkg-tools), are pinned in [`tools/Cargo.toml`](./tools/Cargo.toml) and installed into `target/tools` as needed, or ahead of time with `make tools`. Dependabot bumps the pinned versions.

## Community

### Code of Conduct

The Componentized project follow the [Contributor Covenant Code of Conduct](./CODE_OF_CONDUCT.md). In short, be kind and treat others with respect.

### Communication

General discussion and questions about the project can occur in the project's [GitHub discussions](https://github.com/orgs/componentized/discussions).

### Contributing

The Componentized project team welcomes contributions from the community. A contributor license agreement (CLA) is not required. You own full rights to your contribution and agree to license the work to the community under the Apache License v2.0, via a [Developer Certificate of Origin (DCO)](https://developercertificate.org). For more detailed information, refer to [CONTRIBUTING.md](CONTRIBUTING.md).

## Acknowledgements

This project was conceived in discussion between [Mark Fisher](https://github.com/markfisher) and [Scott Andrews](https://github.com/scothis).

## License

Apache License v2.0: see [LICENSE](./LICENSE) for details.
