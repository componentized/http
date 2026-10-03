# HTTP Components <!-- omit in toc -->

A collection of utility components that remix wasi:http types and interfaces.

- [Components](#components)
- [Build](#build)
- [Community](#community)
  - [Code of Conduct](#code-of-conduct)
  - [Communication](#communication)
  - [Contributing](#contributing)
- [Acknowledgements](#acknowledgements)
- [License](#license)


## Components

- [`client`](./components/client/)

## Build

A [dev container](https://containers.dev) is available that contains the necessary tools and configuration out of the box.

Prereqs:
- a rust toolchain
- [`wasm-tools`](https://github.com/bytecodealliance/wasm-tools)
- [`wkg`](https://github.com/bytecodealliance/wasm-pkg-tools)

```sh
make components
```

The build creates each component in [`components`](./components) into `target/components`, e.g. the client at `target/components/client/client.wasm`, along with `target/components/interface.wasm`, the `componentized:http` WIT package. Each component is also built with debug info, e.g. `target/components/client/client.debug.wasm`.

```sh
make test
```

The WIT dependencies in each `wit/deps` directory are fetched rather than committed, pinned by the `wkg.lock` files. The make targets fetch them as needed. To fetch or update them directly, e.g. before building the Rust components with `cargo`, whose bindings are generated from the WIT:

```sh
make wit
```

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
