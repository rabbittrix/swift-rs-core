import type { LedgerRow } from "./types";

const STORAGE_KEY = "sovereignpay-ledger-v1";
const MAX_ROWS = 500;

export function loadLocalLedger(): LedgerRow[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as LedgerRow[];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

export function saveLocalLedger(rows: LedgerRow[]) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(rows.slice(0, MAX_ROWS)));
}

export function mergeLedgerEntries(existing: LedgerRow[], incoming: LedgerRow[]): LedgerRow[] {
  const seen = new Set(existing.map((row) => row.id));
  const merged = [...incoming.filter((row) => !seen.has(row.id)), ...existing];
  return merged.slice(0, MAX_ROWS);
}
