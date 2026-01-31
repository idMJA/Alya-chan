FROM rust:1.91-trixie AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        ca-certificates \
        build-essential \
    && rm -rf /var/lib/apt/lists/*

COPY . .
RUN cargo build --release

FROM debian:trixie-slim

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/alya-chan /usr/local/bin/alya-chan

ENV RUST_LOG=info

CMD ["/usr/local/bin/alya-chan"]