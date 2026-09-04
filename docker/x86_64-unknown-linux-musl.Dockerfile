# -*- mode: dockerfile -*-
#
ARG BASE_IMAGE=messense/rust-musl-cross:x86_64-musl@sha256:ce75e9174325d4fbb3de85c309e2d7ca29f7500169bc4b5d2c611ff7e86d549a

FROM --platform=$BUILDPLATFORM node:24-alpine@sha256:e67514e5d0f6c46656005e1b693b2ec9d52e80b641307de684d4a015ba7a4eaf AS web-builder
WORKDIR /web
COPY ./webclients/svelte .
RUN apk add --no-cache --virtual .gyp \
        python3 \
        make \
        g++ \
    && npm ci --verbose \
    && apk del .gyp \
    && npm run build



FROM --platform=$BUILDPLATFORM ${BASE_IMAGE} AS builder-prep

# Pin the toolchain instead of relying on whatever the base image ships. A
# separate toolchain is installed because `rustup update` fails on these images:
# stripped docs break rustup's removal of the old clippy component.
ARG RUST_VERSION=1.98.0
RUN rustup toolchain install ${RUST_VERSION} \
        --profile minimal \
        --target x86_64-unknown-linux-musl \
    && rustup default ${RUST_VERSION}

COPY --chown=rust:rust . ./

# Install no-op to cache registry index update
RUN cargo version && rustup --version && rustc --version
RUN cargo fetch
COPY --from=web-builder /web/build webclients/svelte/build

FROM --platform=$BUILDPLATFORM builder-prep AS builder-final
RUN cargo build --release --features web

FROM alpine:latest@sha256:28bd5fe8b56d1bd048e5babf5b10710ebe0bae67db86916198a6eec434943f8b
RUN apk --no-cache add ca-certificates

ARG FILE_TO_COPY=do_ddns
ENV FILE_TO_RUN="${FILE_TO_COPY}"
COPY --from=builder-final \
    /home/rust/src/target/x86_64-unknown-linux-musl/release/$FILE_TO_COPY \
    /usr/local/bin/

CMD "/usr/local/bin/${FILE_TO_RUN}"
