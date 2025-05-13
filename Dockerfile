# Build stage
FROM rust:1.86.0 as builder

WORKDIR /app
COPY . .
RUN apt-get update && apt-get install -y pkg-config libssl-dev
RUN cargo build --release
RUN strip target/release/activity_api

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*
ENV LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu
RUN useradd -m apiuser

COPY --from=builder /app/target/release/activity_api /usr/local/bin/activity_api
COPY .env /app/.env

WORKDIR /app
USER apiuser
CMD ["activity_api"]