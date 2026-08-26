# syntax=docker.io/docker/dockerfile:1.7-labs

FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

LABEL org.opencontainers.image.source=https://github.com/bnb-chain/reth-bsc
LABEL org.opencontainers.image.licenses="MIT OR Apache-2.0"

# Install system dependencies
RUN apt-get update && apt-get -y upgrade && apt-get install -y libclang-dev pkg-config

# Builds a cargo-chef plan
FROM chef AS planner
COPY --exclude=.git --exclude=dist . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Build profile, release by default
ARG BUILD_PROFILE=release
ENV BUILD_PROFILE=$BUILD_PROFILE

# Extra Cargo flags
ARG RUSTFLAGS=""
ENV RUSTFLAGS="$RUSTFLAGS"

# Extra Cargo features
ARG FEATURES=""
ENV FEATURES=$FEATURES

# reth-bsc's main.rs has no `malloc_conf` static, so jemalloc profiling is compiled
# in but inactive unless the config is baked into libjemalloc at build time.
ARG JEMALLOC_SYS_WITH_MALLOC_CONF=""
ENV JEMALLOC_SYS_WITH_MALLOC_CONF=$JEMALLOC_SYS_WITH_MALLOC_CONF

# `[profile.release]` sets `strip = "symbols"`, which leaves the binary with no
# symbol table for the heap profiler to resolve against. Override to "none" when
# collecting profiles; the defaults below reproduce a normal release build.
ARG CARGO_PROFILE_RELEASE_STRIP=symbols
ENV CARGO_PROFILE_RELEASE_STRIP=$CARGO_PROFILE_RELEASE_STRIP
ARG CARGO_PROFILE_RELEASE_DEBUG=none
ENV CARGO_PROFILE_RELEASE_DEBUG=$CARGO_PROFILE_RELEASE_DEBUG

# Builds dependencies
RUN cargo chef cook --profile $BUILD_PROFILE --features "$FEATURES" --recipe-path recipe.json

# Build application
COPY --exclude=dist . .
RUN cargo build --bin reth-bsc --profile $BUILD_PROFILE --features "$FEATURES"

# ARG is not resolved in COPY so we have to hack around it by copying the
# binary to a temporary location
RUN cp /app/target/$BUILD_PROFILE/reth-bsc /app/reth-bsc

# Use Ubuntu as the release image
FROM ubuntu AS runtime
WORKDIR /app

# Copy reth over from the build stage
COPY --from=builder /app/reth-bsc /usr/local/bin

# Copy licenses
COPY LICENSE* ./

EXPOSE 30303 30303/udp 9001 8545 8546
ENTRYPOINT ["/usr/local/bin/reth-bsc"]