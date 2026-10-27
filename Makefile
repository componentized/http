# This repository builds with its own build, its components and cli exist to smoke test it, see
# README.md

# wasi-libc main for WebAssembly/wasi-libc#884, threads that outlive the sync export that spawned them
# corrupt the heap with wasi-sdk 34's sysroot
WASI_LIBC_VERSION := f6e9006376e6f8e8f91ab84fc956be4e345588bc
# the WASI version of the components' WIT, wasi-libc imports 0.3.2
WASI_LIBC_WASI_VERSION := 0.3.0

include build/build.mk

test: check-components validate-components
