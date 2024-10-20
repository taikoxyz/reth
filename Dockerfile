FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app
LABEL org.opencontainers.image.source=https://github.com/paradigmxyz/reth
LABEL org.opencontainers.image.licenses="MIT OR Apache-2.0"

# Install system dependencies
RUN apt-get update && apt-get -y upgrade && apt-get install -y libclang-dev pkg-config git

# Builds a cargo-chef plan
FROM chef AS planner
COPY ./reth/Cargo.lock ./Cargo.lock
COPY ./reth/Cargo.toml ./Cargo.toml
COPY ./reth/crates ./crates
COPY ./reth/bin ./bin
COPY ./reth/examples ./examples
COPY ./reth/testing ./testing
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json /app/reth/recipe.json
COPY ./reth/Cargo.lock ./reth/Cargo.lock
COPY ./reth/Cargo.toml ./reth/Cargo.toml
COPY ./reth/crates ./reth/crates
COPY ./reth/bin ./reth/bin
COPY ./reth/examples ./reth/examples
COPY ./reth/testing ./reth/testing
COPY ./revm ./revm
COPY ./revm-inspectors ./revm-inspectors
WORKDIR /app/reth

# Build profile, release by default
ARG BUILD_PROFILE=release
ENV BUILD_PROFILE $BUILD_PROFILE

# Extra Cargo flags
ARG RUSTFLAGS=""
ENV RUSTFLAGS "$RUSTFLAGS"

# Extra Cargo features
ARG FEATURES=""
ENV FEATURES $FEATURES

# Builds dependencies
RUN cargo chef cook --profile $BUILD_PROFILE --features "$FEATURES" --recipe-path recipe.json
# Build application
RUN cargo build --profile $BUILD_PROFILE --features "$FEATURES" --locked --bin reth

# Copy binaries to a temporary location
RUN cp /app/reth/target/$BUILD_PROFILE/reth /app/reth

# Use Ubuntu as the release image
FROM ubuntu:22.04 AS runtime
WORKDIR /app

# Copy reth and rbuilder binaries over from the build stage
COPY --from=builder /app/reth /usr/local/bin

# Copy licenses
COPY LICENSE-* ./

EXPOSE 30303 30303/udp 9001 8545 8546
ENTRYPOINT ["/usr/local/bin/reth"]
