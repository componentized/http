# HTTP Components <!-- omit in toc -->

A collection of utility components that remix wasi:http types and interfaces.

- [Build](#build)
  - [Components](#components)
- [Community](#community)
  - [Code of Conduct](#code-of-conduct)
  - [Communication](#communication)
  - [Contributing](#contributing)
- [Acknowledgements](#acknowledgements)
- [License](#license)


## Build

A [dev container](https://containers.dev) is available that contains the necessary tools and configuration out of the box.

Prereqs:
- a rust toolchain
- [`wasm-tools`](https://github.com/bytecodealliance/wasm-tools)
- [`wkg`](https://github.com/bytecodealliance/wasm-pkg-tools)
- [`wac`](https://github.com/bytecodealliance/wac)
- [`static-config`](https://github.com/componentized/static-config/)

```sh
make components
```

### Components

- [`gate`](./components/gate/)
- [`gate-client`](./components/gate-client/)
- [`gate-handler`](./components/gate-handler/)
- [`http-client`](./components/http-client/)
- [`latch-abstain-all`](./components/latch-abstain-all/)
- [`latch-deny-all`](./components/latch-deny-all/)
- [`latch-method`](./components/latch-method/)
- [`latch-method-readonly`](./components/latch-method-readonly/)
- [`latch-method-readonly-config`](./components/latch-method-readonly-config/)
- [`latch-n2`](./components/latch-n2/)
- [`latch-n3`](./components/latch-n3/)
- [`latch-n4`](./components/latch-n4/)
- [`latch-n5`](./components/latch-n5/)

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
