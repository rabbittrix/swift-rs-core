"use client";

import { useEffect, useState } from "react";

interface Reserve {
  asset: string;
  backing: number;
  circulating: number;
  escrow: number;
}

interface ChainStatus {
  height: number;
  base_fee: number;
  finalized_transactions: number;
  cbdc_count: number;
  oracle_brl_cny_e8: number;
  reserves: Reserve[];
}

export default function LedgerStatus() {
  const [status, setStatus] = useState<ChainStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const load = async () => {
      try {
        const res = await fetch("/api/swift/chain/status");
        if (!res.ok) {
          setError("Chain status unavailable");
          return;
        }
        setStatus(await res.json());
        setError(null);
      } catch {
        setError("Chain status unavailable");
      }
    };
    load();
    const interval = setInterval(load, 5000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="bg-slate-800/50 backdrop-blur-sm rounded-lg p-6 border border-slate-700">
      <h2 className="text-xl font-semibold text-white mb-4">CBDC ledger</h2>
      {error && <p className="text-amber-300 text-sm">{error}</p>}
      {status && (
        <div className="space-y-3 text-sm">
          <div className="flex justify-between text-slate-300">
            <span>Finalized height</span>
            <span className="text-white font-mono">{status.height}</span>
          </div>
          <div className="flex justify-between text-slate-300">
            <span>CBDCs</span>
            <span className="text-white font-mono">{status.cbdc_count}</span>
          </div>
          <div className="flex justify-between text-slate-300">
            <span>Base fee</span>
            <span className="text-white font-mono">{status.base_fee}</span>
          </div>
          <div className="flex justify-between text-slate-300">
            <span>BRL/CNY oracle</span>
            <span className="text-white font-mono">
              {(status.oracle_brl_cny_e8 / 100_000_000).toFixed(2)}
            </span>
          </div>
          <ul className="pt-2 space-y-1">
            {status.reserves.map((reserve) => (
              <li key={reserve.asset} className="flex justify-between text-slate-400">
                <span>{reserve.asset}</span>
                <span>
                  {reserve.circulating.toLocaleString()} / {reserve.backing.toLocaleString()}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
