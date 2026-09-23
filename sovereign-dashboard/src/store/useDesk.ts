import { create } from "zustand";
import { getNetworkStats, getTransactions, getWalletBalances } from "@/lib/api";
import type { CurrencyBalance, DeskView, LedgerRow, LiveTransaction, NetworkStats } from "@/lib/types";

interface DeskState {
  view: DeskView;
  stats: NetworkStats | null;
  balances: CurrencyBalance[];
  history: LedgerRow[];
  feed: LiveTransaction[];
  tpsSeries: Array<{ t: string; tps: number }>;
  setView: (view: DeskView) => void;
  hydrate: () => Promise<void>;
  pushLive: (tx: LiveTransaction) => void;
  remember: (row: LedgerRow) => void;
}

export const useDesk = create<DeskState>((set) => ({
  view: "simulator",
  stats: null,
  balances: [],
  history: [],
  feed: [],
  tpsSeries: [],
  setView: (view) => set({ view }),
  hydrate: async () => {
    const [stats, balances, history] = await Promise.all([
      getNetworkStats(),
      getWalletBalances(),
      getTransactions(),
    ]);
    const tpsSeries = Array.from({ length: 12 }, (_, index) => ({
      t: `${index + 1}`,
      tps: 4300 + ((index * 137) % 400),
    }));
    set({ stats, balances, history, tpsSeries });
  },
  pushLive: (tx) =>
    set((state) => {
      const tps = 4300 + (tx.settled_ms % 500);
      const series = [...state.tpsSeries, { t: tx.id.slice(-4), tps }].slice(-16);
      return {
        feed: [tx, ...state.feed].slice(0, 8),
        tpsSeries: series,
        stats: state.stats ? { ...state.stats, tps, avg_settlement_ms: tx.settled_ms } : state.stats,
      };
    }),
  remember: (row) => set((state) => ({ history: [row, ...state.history].slice(0, 12) })),
}));
