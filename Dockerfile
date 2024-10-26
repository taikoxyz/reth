FROM lukemathwalker/cargo-chef:latest-rust-1 AS builder
RUN apt-get update && apt-get -y upgrade && apt-get install -y libclang-dev pkg-config git

COPY ./reth/Cargo.lock ./reth/Cargo.lock
COPY ./reth/Cargo.toml ./reth/Cargo.toml
COPY ./reth/crates ./reth/crates
COPY ./reth/bin ./reth/bin
COPY ./reth/examples ./reth/examples
COPY ./reth/testing ./reth/testing
COPY ./revm ./revm
COPY ./revm-inspectors ./revm-inspectors

WORKDIR /reth
RUN cargo build --release --bin reth

FROM ubuntu:22.04 AS runtime
COPY --from=builder /reth/target/release/reth /usr/local/bin

WORKDIR /app
# RUN reth

EXPOSE 30303 30303/udp 9001 8545 8546
ENTRYPOINT ["/usr/local/bin/reth"]
