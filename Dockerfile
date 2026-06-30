FROM debian:bookworm-slim

ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=Europe/Berlin

RUN apt-get update && apt-get install -y \
    bash \
    inotify-tools \
    p7zip-full \
    unrar-free \
    unzip \
    zip \
    curl \
    jq \
    ca-certificates \
    procps \
    findutils \
    coreutils \
    file \
 && apt-get clean \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY watcher.sh /app/
COPY lib /app/lib

RUN chmod +x /app/watcher.sh

ENTRYPOINT ["/app/watcher.sh"]
