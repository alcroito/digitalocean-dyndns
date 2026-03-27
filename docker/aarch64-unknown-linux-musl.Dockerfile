# -*- mode: dockerfile -*-
#
ARG BASE_IMAGE=messense/rust-musl-cross:aarch64-musl@sha256:eab6a58ff66eaa33fa87fc31ed11403596719ca3f23aa51626fb993d77c1200b

FROM --platform=$BUILDPLATFORM node:24-alpine@sha256:01743339035a5c3c11a373cd7c83aeab6ed1457b55da6a69e014a95ac4e4700b AS web-builder
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

FROM --platform=$BUILDPLATFORM builder-prep AS builder-final
RUN cargo build --release --features web

FROM alpine:latest@sha256:25109184c71bdad752c8312a8623239686a9a2071e8825f20acb8f2198c3f659
RUN apk --no-cache add ca-certificates

ARG FILE_TO_COPY=do_ddns
ENV FILE_TO_RUN="${FILE_TO_COPY}"
COPY --from=builder-final \
    /home/rust/src/target/aarch64-unknown-linux-musl/release/$FILE_TO_COPY \
    /usr/local/bin/

CMD "/usr/local/bin/${FILE_TO_RUN}"
