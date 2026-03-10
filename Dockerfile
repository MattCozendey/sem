FROM rust:1-slim-bookworm AS builder

WORKDIR /build

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libgit2-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

COPY crates/ .

RUN cargo build --release -p sem-cli

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    libgit2-1.5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/sem /usr/local/bin/sem

WORKDIR /repo

ENTRYPOINT ["sem"]
