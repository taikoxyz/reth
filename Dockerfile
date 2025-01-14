# Use the latest Rust image for building reth
FROM lukemathwalker/cargo-chef:0.1.68-rust-1 AS chef
WORKDIR /app
LABEL org.opencontainers.image.source=https://github.com/paradigmxyz/reth
LABEL org.opencontainers.image.licenses="MIT OR Apache-2.0"

# Install system dependencies
RUN apt-get update && apt-get -y upgrade && apt-get install -y libclang-dev pkg-config git

# Build a cargo-chef plan
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

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

# Build reth application
COPY . .
RUN cargo build --profile $BUILD_PROFILE --features "$FEATURES" --locked --bin reth

# Clone rbuilder repository to access its config files
RUN git clone -b gwyneth https://github.com/taikoxyz/rbuilder.git /app/rbuilder

# Copy reth binary to a temporary location
RUN cp /app/target/$BUILD_PROFILE/reth /app/reth

# Use Ubuntu as the final runtime image
FROM ubuntu:22.04 AS runtime
WORKDIR /app

# Install necessary runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Install Rust and Cargo
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Copy reth binary from the build stage
COPY --from=builder /app/reth /usr/local/bin

# Copy rbuilder binary from the published Docker Hub image
COPY --from=gwynethtaiko/rbuilder:latest /usr/local/bin/rbuilder /usr/local/bin/rbuilder

# Copy rbuilder repository (configs, etc.) from the build stage
COPY --from=builder /app/rbuilder /app/rbuilder

# Copy licenses
COPY LICENSE-* ./

# Create start script for rbuilder
RUN echo '#!/bin/bash\nrbuilder run /app/rbuilder/config-gwyneth-reth.toml' > /app/start_rbuilder.sh && \
    chmod +x /app/start_rbuilder.sh

# Expose necessary ports
EXPOSE 30303 30303/udp 9001 8545 8546

ENV RUST_BACKTRACE=full

# Set reth as the entrypoint
ENTRYPOINT ["/usr/local/bin/reth"]