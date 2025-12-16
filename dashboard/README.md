# Swift-RS Dashboard

Modern Next.js dashboard for monitoring the Swift-RS financial messaging engine.

## Features

- **Real-time Payment Flows**: Live stream of payment messages (MT/MX)
- **AI Risk Scoring**: Fraud detection and anomaly analysis visualization
- **Latency Metrics**: P50, P95, P99 latency tracking with <5ms target
- **System Status**: Gateway health monitoring and connection status

## Getting Started

### Prerequisites

- Node.js 18+ and npm
- Swift-RS Gateway running on `http://localhost:8080`

### Installation

```bash
cd dashboard
npm install
```

### Development

```bash
npm run dev
```

Open [http://localhost:3000](http://localhost:3000) in your browser.

### Build for Production

```bash
npm run build
npm start
```

## Architecture

- **Next.js 14** with App Router
- **TypeScript** for type safety
- **Tailwind CSS** for styling
- **Recharts** for data visualization
- **Real-time updates** via polling (can be upgraded to WebSockets)

## API Integration

The dashboard connects to the Swift-RS Gateway API via proxy configured in `next.config.js`:

- Health check: `/api/swift/health`
- Messages: `/api/swift/messages`

## License

Apache-2.0
