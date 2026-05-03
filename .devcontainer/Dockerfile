# Devcontainer image for evervault-rs.
# Published to ghcr.io/<owner>/evervault-rs/devcontainer:latest on merges to main.
FROM mcr.microsoft.com/devcontainers/rust:1-bookworm

RUN rustup component add rustfmt clippy

# Pre-fetch dependencies so the first cargo build inside the container is fast.
WORKDIR /workspaces/evervault-rs
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && echo 'fn main() {}' > src/main.rs \
    && cargo fetch \
    && rm -rf src target
