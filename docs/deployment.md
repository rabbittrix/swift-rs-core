# Deployment

## Local

```bash
cargo test --workspace --release
cargo run --bin swift-rs-gateway
```

The gateway listens on `http://0.0.0.0:8080`. Prometheus metrics are at `/metrics`.

Demo parties: `br-cb`, `cn-cb`, `in-cb`, `ae-cb`, `eu-cb`, `br-bank`, `cn-bank`, `in-bank`, `ae-bank`, `eu-bank`, `escrow`, `restricted`, `timelock`.

```bash
curl -s http://localhost:8080/api/v1/chain/status
curl -s -X POST http://localhost:8080/api/v1/transfers \
  -H "content-type: application/json" \
  -d "{\"from\":\"br-bank\",\"to\":\"cn-bank\",\"asset\":\"cbdc:brl\",\"amount\":1000,\"purpose\":\"trade\"}"
```

## Docker and Kubernetes

`docker-compose up -d` builds the gateway from the Dockerfile. The Kubernetes deployment in `k8s/deployment.yaml` expects the image `swift-rs-gateway:latest` and probes `/health`.

Scrape `/metrics` with `monitoring/prometheus.yml`. Alerting rules are in `monitoring/alerts.yml`. Import `monitoring/grafana-dashboard.json` into Grafana.

## Load

`loadtest/k6.js` hits `/health` and `/api/v1/chain/status`. It checks that the process stays up. It is not a 10,000 TPS certification.
