###
# ---- Builder Stage ----
###
FROM rust:1.85-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    capnproto libcapnp-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY pool-apps/ pool-apps/
COPY miner-apps/ miner-apps/
COPY bitcoin-core-sv2/ bitcoin-core-sv2/
COPY stratum-apps/ stratum-apps/

# Build target app
ARG APP
RUN echo "Building ${APP}" && \
    case "$APP" in \
        pool_sv2) cargo build --release --manifest-path pool-apps/pool/Cargo.toml --target-dir ./ ;; \
        jd_server) cargo build --release --manifest-path pool-apps/jd-server/Cargo.toml --target-dir ./ ;; \
        jd_client_sv2) cargo build --release --manifest-path miner-apps/jd-client/Cargo.toml --target-dir ./ ;; \
        translator_sv2) cargo build --release --manifest-path miner-apps/translator/Cargo.toml --target-dir ./ ;; \
        *) echo "Unknown APP: $APP" && exit 1 ;; \
    esac

###
# ---- Runtime Stage ----
###
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    gettext-base && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app
ARG APP
ENV APP=${APP}

COPY --from=builder /app/release/${APP} /app/${APP}

ENTRYPOINT ["/bin/sh", "-c", "/app/${APP}"]
