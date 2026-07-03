FROM rust:1-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked


FROM debian:bookworm-slim

ENV TZ=Europe/Berlin

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    p7zip-full \
    unzip \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/xdcc-extractor /usr/local/bin/xdcc-extractor

ENTRYPOINT ["/usr/local/bin/xdcc-extractor"]
