# Multi-stage build that produces a small runtime image containing the
# release binary. Used by the `images` workflow to publish ghcr.io/<owner>/evervault-rs:latest.
FROM rust:1-bookworm AS builder
WORKDIR /src

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/evervault-rs /usr/local/bin/evervault-rs

ENTRYPOINT ["/usr/local/bin/evervault-rs"]
