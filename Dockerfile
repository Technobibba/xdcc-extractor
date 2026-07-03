FROM rust:1-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked


FROM debian:bookworm-slim

ENV TZ=Europe/Berlin

RUN set -eux; \
    if [ -f /etc/apt/sources.list.d/debian.sources ]; then \
        sed -i 's/Components: main/Components: main contrib non-free non-free-firmware/g' /etc/apt/sources.list.d/debian.sources; \
    elif [ -f /etc/apt/sources.list ]; then \
        sed -i 's/ main/ main contrib non-free non-free-firmware/g' /etc/apt/sources.list; \
    fi; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        p7zip-full \
        unrar \
        unzip \
        tar \
        gzip \
        xz-utils \
        bzip2 \
    ; \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/xdcc-extractor /usr/local/bin/xdcc-extractor

ENTRYPOINT ["/usr/local/bin/xdcc-extractor"]
