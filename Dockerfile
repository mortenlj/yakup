ARG binary_name=yakup

FROM --platform=$BUILDPLATFORM ghcr.io/mortenlj/mise-lib/rust-builder:latest AS build

FROM ghcr.io/mortenlj/mise-lib/rust:latest AS docker
CMD ["/usr/bin/yakup"]
