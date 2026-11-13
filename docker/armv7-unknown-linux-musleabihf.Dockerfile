# -*- mode: dockerfile -*-
#
ARG BASE_IMAGE=messense/rust-musl-cross:armv7-musleabihf@sha256:dd494898259cf6981f9832bfd4437ddfc8393605164b3c242e4efe8a6bc30dfb

FROM --platform=$BUILDPLATFORM node:24-alpine@sha256:116b800c3754b440ee0aa3ed2f20f5bf3d99a554bfd9adeea09ea8017f6e85f5 AS web-builder
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

# Pin the toolchain instead of relying on whatever the base image ships. A
# separate toolchain is installed because `rustup update` fails on these images:
# stripped docs break rustup's removal of the old clippy component.
ARG RUST_VERSION=1.98.0
RUN rustup toolchain install ${RUST_VERSION} \
        --profile minimal \
        --target armv7-unknown-linux-musleabihf \
    && rustup default ${RUST_VERSION}

COPY --chown=rust:rust . ./

# Install no-op to cache registry index update
RUN cargo version && rustup --version && rustc --version
RUN cargo fetch
COPY --from=web-builder /web/build webclients/svelte/build

FROM --platform=$BUILDPLATFORM builder-prep AS builder-final
RUN cargo build --release --features web

FROM alpine:latest@sha256:f59abe656b692d901e184be3445e084786602108599d2a51d229c0f5a48c71bf
RUN apk --no-cache add ca-certificates

ARG FILE_TO_COPY=do_ddns
ENV FILE_TO_RUN="${FILE_TO_COPY}"
COPY --from=builder-final \
    /home/rust/src/target/armv7-unknown-linux-musleabihf/release/$FILE_TO_COPY \
    /usr/local/bin/

CMD "/usr/local/bin/${FILE_TO_RUN}"
