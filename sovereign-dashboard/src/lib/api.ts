import type {
  CurrencyBalance,
  LedgerRow,
  LiveTransaction,
  NetworkStats,
  PaymentRequest,
  PaymentStatus,
  SimulatePaymentRequest,
  TransactionUpdate,
  ValidatorNode,
} from "./types";

const VALIDATORS: ValidatorNode[] = [
  { id: "br", name: "Banco Central do Brasil", city: "Brasília", x: 30, y: 68 },
  { id: "cn", name: "PBOC", city: "Beijing", x: 78, y: 38 },
  { id: "ae", name: "Central Bank of the UAE", city: "Abu Dhabi", x: 62, y: 48 },
  { id: "in", name: "Reserve Bank of India", city: "Mumbai", x: 70, y: 54 },
  { id: "eu", name: "ECB", city: "Frankfurt", x: 48, y: 36 },
];

const PAIRS: Array<[string, string, string]> = [
  ["Banco Central do Brasil", "PBOC", "BRL-CBDC"],
  ["PBOC", "Central Bank of the UAE", "CNY-CBDC"],
  ["Reserve Bank of India", "ECB", "INR-CBDC"],
  ["ECB", "Banco Central do Brasil", "EUR-CBDC"],
  ["Central Bank of the UAE", "Reserve Bank of India", "AED-CBDC"],
];

let tick = 0;

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function restricted(name: string): boolean {
  const value = name.toLowerCase();
  return value.includes("restricted") || value.includes("sanction");
}

function demoHash(seed: string): string {
  let acc = 0xcbf29ce484222325n;
  for (const byte of new TextEncoder().encode(seed)) {
    acc ^= BigInt(byte);
    acc = (acc * 0x100000001b3n) & ((1n << 64n) - 1n);
  }
  return acc.toString(16).padStart(16, "0").repeat(2);
}

export function mockPayment(payload: PaymentRequest): PaymentStatus {
  if (payload.amount <= 0) {
    throw new Error("amount must be positive");
  }
  const sovereign = payload.route === "SOVEREIGN";
  const id = `pay-${Date.now()}`;
  if (restricted(payload.from) || restricted(payload.to)) {
    return {
      id,
      route: payload.route,
      status: "blocked",
      fee_usd: sovereign ? 0 : 45,
      settlement_label: "not settled",
      tx_hash: null,
      detail: "counterparty failed the sanctions screen",
    };
  }
  if (sovereign) {
    return {
      id,
      route: "SOVEREIGN",
      status: "settled",
      fee_usd: 0.01,
      settlement_label: "< 2 seconds",
      tx_hash: demoHash(id),
      detail: "atomic CBDC swap finalized by the local consortium",
    };
  }
  return {
    id,
    route: "LEGACY",
    status: "settled",
    fee_usd: 45,
    settlement_label: "2–3 days",
    tx_hash: null,
    detail: "correspondent chain and USD clearing",
  };
}

export function mockStats(): NetworkStats {
  return {
    tps: 4500,
    total_volume_24h: "1.2B USD",
    active_nodes: 42,
    avg_settlement_ms: 840,
    validators: VALIDATORS,
  };
}

export function mockBalances(): CurrencyBalance[] {
  return [
    { currency: "BRL-CBDC", name: "Digital Real", amount: "48,250,000.00", issuer: "Banco Central do Brasil" },
    { currency: "CNY-CBDC", name: "Digital Yuan", amount: "36,800,000.00", issuer: "PBOC" },
    { currency: "AED-CBDC", name: "Digital Dirham", amount: "12,400,000.00", issuer: "Central Bank of the UAE" },
    { currency: "INR-CBDC", name: "Digital Rupee", amount: "9,100,000.00", issuer: "Reserve Bank of India" },
    { currency: "EUR-CBDC", name: "Digital Euro", amount: "6,750,000.00", issuer: "ECB" },
  ];
}

export function mockActivity(): LedgerRow[] {
  return [
    { id: "hx-9f21", date: "23 Sep 2026 17:41", type: "Swap", amount: "5,000,000.00", currency: "BRL-CBDC", status: "Settled", tx_hash: demoHash("swap-1"), verified: true },
    { id: "hx-9f18", date: "23 Sep 2026 16:05", type: "Receive", amount: "1,200,000.00", currency: "CNY-CBDC", status: "Settled", tx_hash: demoHash("rcv-1"), verified: true },
    { id: "hx-9f11", date: "23 Sep 2026 11:22", type: "Send", amount: "800,000.00", currency: "AED-CBDC", status: "Settled", tx_hash: demoHash("snd-1"), verified: true },
    { id: "hx-9e90", date: "22 Sep 2026 19:14", type: "Swap", amount: "250,000.00", currency: "INR-CBDC", status: "Blocked", tx_hash: demoHash("blk-1"), verified: false },
    { id: "hx-9e44", date: "22 Sep 2026 09:03", type: "Receive", amount: "2,400,000.00", currency: "EUR-CBDC", status: "Settled", tx_hash: demoHash("rcv-2"), verified: true },
  ];
}

export function mockLive(): LiveTransaction {
  const [from_node, to_node, currency] = PAIRS[tick % PAIRS.length];
  tick += 1;
  const amount = (1_250_000 + (tick % 7) * 175_000).toLocaleString("en-US");
  return {
    id: `live-${tick.toString(16)}`,
    from_node,
    to_node,
    amount,
    currency,
    settled_ms: 640 + (tick % 5) * 40,
  };
}

export async function initiatePayment(payload: PaymentRequest): Promise<PaymentStatus> {
  if (!isTauri()) return mockPayment(payload);
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<PaymentStatus>("initiate_payment", { payload });
}

export async function getNetworkStats(): Promise<NetworkStats> {
  if (!isTauri()) return mockStats();
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<NetworkStats>("get_network_stats");
}

export async function getWalletBalances(): Promise<CurrencyBalance[]> {
  if (!isTauri()) return mockBalances();
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<CurrencyBalance[]>("get_wallet_balances");
}

export async function getTransactions(): Promise<LedgerRow[]> {
  if (!isTauri()) return mockActivity();
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<LedgerRow[]>("get_transactions");
}

export async function runSimulatePayment(request: SimulatePaymentRequest): Promise<string> {
  if (!isTauri()) {
    throw new Error("browser simulation uses simulatePaymentBrowser");
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("simulate_payment", { request });
}

export async function subscribeTransactionUpdates(
  onUpdate: (update: TransactionUpdate) => void,
): Promise<() => void> {
  if (!isTauri()) {
    return () => {};
  }
  const { listen } = await import("@tauri-apps/api/event");
  return listen<TransactionUpdate>("transaction_update", (event) => onUpdate(event.payload));
}

export async function subscribeLive(onTx: (tx: LiveTransaction) => void): Promise<() => void> {
  if (!isTauri()) {
    const timer = window.setInterval(() => onTx(mockLive()), 3000);
    return () => window.clearInterval(timer);
  }
  const { listen } = await import("@tauri-apps/api/event");
  return listen<LiveTransaction>("live-transaction", (event) => onTx(event.payload));
}
