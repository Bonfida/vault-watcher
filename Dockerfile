FROM lukemathwalker/cargo-chef:latest-rust-bookworm AS chef
WORKDIR vault-watcher

FROM chef as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /vault-watcher/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim AS base
RUN apt-get update
RUN apt-get install -y ca-certificates

FROM base AS runtime
WORKDIR vault-watcher
COPY --from=builder /vault-watcher/target/release/vault-watcher /usr/local/bin
COPY config.json accounts.json ./
ENTRYPOINT ["/usr/local/bin/vault-watcher", "accounts.json", "config.json"]