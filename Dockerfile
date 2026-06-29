FROM alpine:3.20

RUN apk add --no-cache \
    bash \
    7zip \
    inotify-tools \
    coreutils \
    findutils \
    grep \
    sed \
    util-linux \
    file \
    tzdata

ENV TZ=Europe/Berlin

COPY watcher.sh /

RUN chmod +x /watcher.sh

ENTRYPOINT ["/watcher.sh"]
