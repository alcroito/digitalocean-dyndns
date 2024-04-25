# -*- mode: dockerfile -*-
#
ARG BASE_IMAGE=messense/rust-musl-cross:aarch64-musl

FROM --platform=$BUILDPLATFORM node:20-alpine@sha256:7a91aa397f2e2dfbfcdad2e2d72599f374e0b0172be1d86eeb73f1d33f36a4b2 AS web-builder
WORKDIR /web
COPY ./webclients/svelte .
RUN apk add --no-cache --virtual .gyp \
        python3 \
        make \
        g++ \
    && npm ci --verbose \
    && apk del .gyp \
    && npm run build

# BUILDPLATFORM forces the build stage to be done on linux-amd64
# regardless of the specified target platform in the final stage.
FROM --platform=$BUILDPLATFORM ${BASE_IMAGE} AS builder-prep

COPY --chown=rust:rust . ./

# Install no-op to cache registry index update
RUN cargo version && rustup --version && rustc --version
RUN cargo fetch
COPY --from=web-builder /web/build webclients/svelte/build

FROM --platform=$BUILDPLATFORM builder-prep as builder-final
RUN cargo build --release --features web

FROM alpine:latest
RUN apk --no-cache add ca-certificates

ARG FILE_TO_COPY=do_ddns
ENV FILE_TO_RUN="${FILE_TO_COPY}"
COPY --from=builder-final \
    /home/rust/src/target/aarch64-unknown-linux-musl/release/$FILE_TO_COPY \
    /usr/local/bin/

CMD "/usr/local/bin/${FILE_TO_RUN}"
