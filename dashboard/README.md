# Swift-RS operator dashboard

Next.js view of the gateway: health, ledger reserves, payment flows, risk scores, and latency.

This is the operator console. The side-by-side legacy and sovereign demonstration lives in [`../sovereign-dashboard`](../sovereign-dashboard/README.md).

## Features

- **Gateway health** polled from `/api/swift/health`
- **Ledger status**: height, CBDC reserves, and the BRL/CNY oracle, from `/api/swift/chain/status`
- **Payment flows** for messages accepted by the gateway
- **Risk scoring** from the AI scorer
- **Latency** percentiles reported by the metrics panel

## Prerequisites

- Node.js 18+
- Gateway on [http://localhost:8080](http://localhost:8080)

```bash
# from the repo root
cargo run --bin swift-rs-gateway
```

## Run

```bash
cd dashboard
npm install
npm run dev
```

Open [http://localhost:3000](http://localhost:3000).

```bash
npm run build
npm start
```

## How it talks to the gateway

`next.config.js` rewrites `/api/swift/*` to the gateway:

| Browser path | Gateway |
| --- | --- |
| `/api/swift/health` | `GET /health` |
| `/api/swift/chain/status` | `GET /api/v1/chain/status` |
| `/api/swift/messages` | `POST /api/v1/messages` |
| `/api/swift/transfers` | `POST /api/v1/transfers` |
| `/api/swift/swaps` | `POST /api/v1/swaps` |

`NEXT_PUBLIC_GATEWAY_URL` overrides the gateway origin. The default is `http://localhost:8080`.

## Stack

Next.js 14 (App Router), TypeScript, Tailwind CSS, Recharts. The ledger panel polls; it does not open a WebSocket.

## License

Apache-2.0
