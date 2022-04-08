FROM rust:latest

COPY ./ ./

RUN cargo build --release

CMD ["./target/release/vault-watcher", "accounts.json", "config.json"]