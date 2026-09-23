export type RouteKind = "LEGACY" | "SOVEREIGN";
export type RouteType = "LegacySwift" | "SovereignRs";
export type DeskView = "simulator" | "network" | "wallet" | "compliance";
export type StepState = "idle" | "active" | "done" | "blocked";
export type PaymentState = "idle" | "running" | "settled" | "blocked";

export interface PaymentRequest {
  from: string;
  to: string;
  amount: number;
  currency_from: string;
  currency_to: string;
  route: RouteKind;
}

export interface SimulatePaymentRequest {
  paymentId?: string;
  from: string;
  to: string;
  amount: number;
  currencyFrom: string;
  currencyTo: string;
  route: RouteType;
}

export interface TransactionStep {
  index: number;
  total: number;
  label: string;
  detail: string;
  tone: "amber" | "emerald" | "blue" | "rose";
  status: "active" | "done" | "blocked";
}

export interface TransactionUpdate {
  paymentId: string;
  route: RouteType;
  kind: "step" | "complete" | "blocked";
  step: TransactionStep | null;
  elapsedMs: number;
  feeUsd: number | null;
  txHash: string | null;
  totalElapsedMs: number | null;
  timestamp: string;
}

export interface TrackSnapshot {
  phase: PaymentState;
  steps: TransactionStep[];
  feeUsd: number | null;
  txHash: string | null;
  totalMs: number | null;
}

export interface PaymentStatus {
  id: string;
  route: string;
  status: "settled" | "blocked";
  fee_usd: number;
  settlement_label: string;
  tx_hash: string | null;
  detail: string;
}

export interface ValidatorNode {
  id: string;
  name: string;
  city: string;
  x: number;
  y: number;
}

export interface NetworkStats {
  tps: number;
  total_volume_24h: string;
  active_nodes: number;
  avg_settlement_ms: number;
  validators: ValidatorNode[];
}

export interface CurrencyBalance {
  currency: string;
  name: string;
  amount: string;
  issuer: string;
}

export interface LedgerRow {
  id: string;
  date: string;
  type: "Send" | "Receive" | "Swap";
  amount: string;
  currency: string;
  status: "Settled" | "Blocked";
  tx_hash: string;
  verified?: boolean;
}

export interface LiveTransaction {
  id: string;
  from_node: string;
  to_node: string;
  amount: string;
  currency: string;
  settled_ms: number;
}

export interface TrackStep {
  label: string;
  detail: string;
  state: StepState;
}
