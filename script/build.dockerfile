# syntax=docker/dockerfile:1

FROM rust:1.76.0-slim

RUN DEBIAN_FRONTEND=noninteractive apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y \
  libssl-dev \
  pkg-config \
  tree \
  && rm -rf /var/lib/apt/lists/*

USER 1000:1000

RUN mkdir /tmp/build

COPY --link --chmod=555 src /tmp/build/src
COPY --link --chmod=444 Cargo.toml /tmp/build/Cargo.toml
COPY --link --chmod=444 Cargo.lock /tmp/build/Cargo.lock

RUN --mount=type=cache,target=/tmp/build/target

WORKDIR /tmp/build

RUN set -x && tree -l /tmp/build && cargo build --release && cp target/release/pkg-info-updater /tmp/build/pkg-info-updater
