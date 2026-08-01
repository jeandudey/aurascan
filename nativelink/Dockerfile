FROM ghcr.io/tracemachina/nativelink:v1.6.1 AS upstream

FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends \
    coreutils ca-certificates unzip xz-utils \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /data/nativelink
COPY --from=upstream /nix/store/sd3glc0c4avl0gabbcjw48pzcn72qrbf-nativelink-x86_64-unknown-linux-musl-1.6.1/bin/nativelink /usr/local/bin/nativelink
ENTRYPOINT ["/usr/local/bin/nativelink"]
