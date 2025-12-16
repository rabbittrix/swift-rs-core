# ⚡ Swift-RS: Next-Gen Financial Messaging Engine

> _Breaking the Java Hegemony with Rust Performance, Safety, and AI Intelligence._

[![Rust](https://img.shields.io/badge/built_with-Rust-orange?logo=rust)](https://www.rust-lang.org/)
[![Architecture](https://img.shields.io/badge/pattern-CQRS%2FES-blue)](https://martinfowler.com/bliki/CQRS.html)
[![Standard](https://img.shields.io/badge/ISO-20022%20%7C%20MT%2FMX-compliant-green)](https://www.iso20022.org/)
[![Docker](https://img.shields.io/badge/deployment-Kubernetes-blue?logo=kubernetes)](https://kubernetes.io/)

## 🚀 Quick Start

```bash
# Build all crates
cargo build --release

# Run the API Gateway
cargo run --bin swift-rs-gateway

# Run tests
cargo test --workspace
```

## 📦 Project Structure

This is a Rust workspace containing the following crates:

- **swift-rs-core**: Domain logic and message validation
- **swift-rs-iso20022**: High-performance ISO 20022 (MX) serializer/deserializer
- **swift-rs-connector**: Secure connectivity layer (VPN/SNA/AMQP)
- **swift-rs-cqrs**: Lightweight Event Sourcing implementation
- **swift-rs-gateway**: API Gateway (gRPC/REST)
- **swift-rs-ai**: AI/ML inference layer for fraud detection

## 🏗 Architecture

See [.project_information.md](.project_information.md) for detailed architecture documentation.

## 🛡️ Security & Compliance

- Memory Safety: Rust guarantees protection against buffer overflows
- Encryption: TLS 1.3 and AES-256
- Audit: Every state change is an immutable event

## 📄 License

Apache-2.0

## 👤 Author

Roberto de Souza <rabbittrix@hotmail.com>
