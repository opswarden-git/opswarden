# --- server/Dockerfile ---

# ---- build ----
FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml ./
COPY server ./server
RUN cargo build --release -p opswarden-server

# ---- runtime ----
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates wget \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/opswarden-server /usr/local/bin/opswarden-server
EXPOSE 8080
CMD ["opswarden-server"]
