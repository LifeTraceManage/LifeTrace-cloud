# syntax=docker/dockerfile:1

FROM rust:1.88-slim AS rust-builder
WORKDIR /build
ENV CARGO_NET_RETRY=3 \
    CARGO_HTTP_TIMEOUT=60 \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    sh -ec 'for attempt in 1 2 3 4 5; do \
      cargo fetch --locked --manifest-path Cargo.toml && exit 0; \
      echo "cargo fetch failed (attempt ${attempt}/5), retrying in 5s..." >&2; \
      sleep 5; \
    done; exit 1'

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --offline --locked --release --bin lifetrace-cloud

FROM debian:bookworm-slim
RUN sed -i \
        -e 's|deb.debian.org/debian-security|mirrors.aliyun.com/debian-security|g' \
        -e 's|deb.debian.org/debian|mirrors.aliyun.com/debian|g' \
        /etc/apt/sources.list.d/debian.sources \
    && apt-get -o Acquire::Retries=3 update \
    && apt-get install -y --no-install-recommends ca-certificates curl libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 lifetrace \
    && mkdir -p /data/photo-staging /app/web \
    && chown -R lifetrace:lifetrace /data /app

WORKDIR /app
COPY --from=rust-builder /build/target/release/lifetrace-cloud /app/lifetrace-cloud
# LifeTrace-web lives in its own private repository. Production Compose mounts
# the independently built dist directory read-only at /app/web.
ENV LIFETRACE_DATABASE_PATH=/data/lifetrace.db \
    LIFETRACE_WEB_ROOT=/app/web \
    PHOTO_STAGING_DIR=/data/photo-staging

USER lifetrace
EXPOSE 8787 8869
VOLUME ["/data"]

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl --fail --silent http://127.0.0.1:8787/health/ready || exit 1

ENTRYPOINT ["/app/lifetrace-cloud"]
