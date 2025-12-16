# Multi-stage build for Swift-RS Gateway
FROM rust:1.75 as builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY swift-rs-core ./swift-rs-core
COPY swift-rs-iso20022 ./swift-rs-iso20022
COPY swift-rs-connector ./swift-rs-connector
COPY swift-rs-cqrs ./swift-rs-cqrs
COPY swift-rs-gateway ./swift-rs-gateway
COPY swift-rs-ai ./swift-rs-ai

# Build release
RUN cargo build --release --bin swift-rs-gateway

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/swift-rs-gateway /app/swift-rs-gateway

EXPOSE 8080

CMD ["/app/swift-rs-gateway"]

