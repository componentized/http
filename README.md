# HTTP Components <!-- omit in toc -->

A collection of utility components that remix wasi:http types and interfaces.

- [Components](#components)
  - [Choosing a component variant](#choosing-a-component-variant)
- [Build](#build)
- [Community](#community)
  - [Code of Conduct](#code-of-conduct)
  - [Communication](#communication)
  - [Contributing](#contributing)
- [Acknowledgements](#acknowledgements)
- [License](#license)


## Components

- [`client`](./components/client/)
- [`trace`](./components/trace/)
- [`trace-client`](./components/trace-client/)
- [`trace-handler`](./components/trace-handler/)
- [`trace-componentized-client`](./components/trace-componentized-client/)
- [`trace-types`](./components/trace-types/)

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
