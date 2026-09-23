import type {
  CountryNode,
  LedgerRow,
  LiveTransaction,
  RouteType,
  TrackSnapshot,
  TransactionUpdate,
} from "./types";
import { money } from "./utils";

function bindingHash(paymentId: string, route: RouteType): string {
  const seed = `${paymentId}:${route}`;
  let acc = 0xcbf29ce484222325n;
  for (const byte of new TextEncoder().encode(seed)) {
    acc ^= BigInt(byte);
    acc = (acc * 0x100000001b3n) & ((1n << 64n) - 1n);
  }
  return `0x${acc.toString(16).padStart(16, "0").repeat(2)}`;
}

export function ledgerRowFromSimulation(
  paymentId: string,
  route: RouteType,
  update: TransactionUpdate,
  origin: CountryNode,
  dest: CountryNode,
  amount: number,
): LedgerRow {
  const settled = update.kind === "complete";
  return {
    id: `${paymentId}-${route === "LegacySwift" ? "legacy" : "sovereign"}`,
    date: new Date().toLocaleString("en-GB", { dateStyle: "medium", timeStyle: "short" }),
    type: "Swap",
    amount: money(amount),
    currency: `${origin.fiatCurrency}-CBDC`,
    status: settled ? "Settled" : "Blocked",
    tx_hash: update.txHash ?? bindingHash(paymentId, route),
    verified: route === "SovereignRs" && settled,
    route,
    corridor: `${origin.name} (${origin.fiatCurrency}) → ${dest.name} (${dest.fiatCurrency})`,
  };
}

export function ledgerRowFromTrack(
  paymentId: string,
  route: RouteType,
  track: TrackSnapshot,
  origin: CountryNode,
  dest: CountryNode,
  amount: number,
): LedgerRow | null {
  if (track.phase !== "settled" && track.phase !== "blocked") return null;
  const settled = track.phase === "settled";
  return {
    id: `${paymentId}-${route === "LegacySwift" ? "legacy" : "sovereign"}`,
    date: new Date().toLocaleString("en-GB", { dateStyle: "medium", timeStyle: "short" }),
    type: "Swap",
    amount: money(amount),
    currency: `${origin.fiatCurrency}-CBDC`,
    status: settled ? "Settled" : "Blocked",
    tx_hash: track.txHash ?? bindingHash(paymentId, route),
    verified: route === "SovereignRs" && settled,
    route,
    corridor: `${origin.name} (${origin.fiatCurrency}) → ${dest.name} (${dest.fiatCurrency})`,
  };
}

export function ledgerRowFromLive(tx: LiveTransaction): LedgerRow {
  return {
    id: tx.id,
    date: new Date().toLocaleString("en-GB", { dateStyle: "medium", timeStyle: "short" }),
    type: "Swap",
    amount: tx.amount,
    currency: tx.currency,
    status: "Settled",
    tx_hash: bindingHash(tx.id, "SovereignRs"),
    verified: true,
    route: "Network",
    corridor: `${tx.from_node} → ${tx.to_node}`,
  };
}
