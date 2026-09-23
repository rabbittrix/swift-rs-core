# Multi-stage build for Swift-RS Gateway
FROM rust:1.88-bookworm AS builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY swift-rs-core ./swift-rs-core
COPY swift-rs-iso20022 ./swift-rs-iso20022
COPY swift-rs-connector ./swift-rs-connector
COPY swift-rs-cqrs ./swift-rs-cqrs
COPY swift-rs-gateway ./swift-rs-gateway
COPY swift-rs-ai ./swift-rs-ai
COPY swift-rs-blockchain ./swift-rs-blockchain
COPY swift-rs-cbdc ./swift-rs-cbdc
COPY swift-rs-tokenization ./swift-rs-tokenization
COPY swift-rs-privacy ./swift-rs-privacy
COPY swift-rs-bridge ./swift-rs-bridge
COPY swift-rs-governance ./swift-rs-governance
COPY swift-rs-economics ./swift-rs-economics

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

