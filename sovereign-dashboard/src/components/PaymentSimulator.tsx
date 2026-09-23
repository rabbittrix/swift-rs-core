import { useCallback, useEffect, useRef, useState } from "react";
import { motion } from "framer-motion";
import { ArrowRight, Zap } from "lucide-react";
import { Button } from "@/components/ui/button";
import { SettlementPipeline } from "@/components/SettlementPipeline";
import { isTauri, runSimulatePayment, subscribeTransactionUpdates } from "@/lib/api";
import { applyUpdate, emptyTrack, simulatePaymentBrowser } from "@/lib/simulation";
import type { SimulatePaymentRequest, TrackSnapshot, TransactionUpdate } from "@/lib/types";
import { money } from "@/lib/utils";
import { useDesk } from "@/store/useDesk";

const INSTITUTIONS = [
  "Banco do Brasil",
  "Bank of China",
  "First Abu Dhabi Bank",
  "State Bank of India",
  "Deutsche Bundesbank desk",
  "Restricted Desk",
];

export function PaymentSimulator() {
  const remember = useDesk((state) => state.remember);
  const [from, setFrom] = useState(INSTITUTIONS[0]);
  const [to, setTo] = useState(INSTITUTIONS[1]);
  const [amount, setAmount] = useState(10_000_000);
  const [running, setRunning] = useState(false);
  const [legacy, setLegacy] = useState<TrackSnapshot>(emptyTrack());
  const [sovereign, setSovereign] = useState<TrackSnapshot>(emptyTrack());
  const activePayment = useRef<string | null>(null);

  const dispatch = useCallback((update: TransactionUpdate) => {
    if (activePayment.current && update.paymentId !== activePayment.current) return;
    if (update.route === "LegacySwift") setLegacy((track) => applyUpdate(track, update));
    else setSovereign((track) => applyUpdate(track, update));
  }, []);

  useEffect(() => {
    let stop = () => {};
    void subscribeTransactionUpdates(dispatch).then((unlisten) => {
      stop = unlisten;
    });
    return () => stop();
  }, [dispatch]);

  async function launch(id: string, body: Omit<SimulatePaymentRequest, "paymentId">, sink: (u: TransactionUpdate) => void) {
    const request: SimulatePaymentRequest = { ...body, paymentId: id };
    if (isTauri()) {
      await runSimulatePayment(request);
      return;
    }
    await simulatePaymentBrowser(request, sink);
  }

  async function send() {
    setRunning(true);
    const id = crypto.randomUUID();
    activePayment.current = id;
    setLegacy({ ...emptyTrack(), phase: "running" });
    setSovereign({ ...emptyTrack(), phase: "running" });
    const base = {
      from,
      to,
      amount,
      currencyFrom: "BRL-CBDC",
      currencyTo: "CNY-CBDC",
    };
    let sovereignHash: string | null = null;
    const sink = (update: TransactionUpdate) => {
      dispatch(update);
      if (update.paymentId === id && update.route === "SovereignRs" && update.kind === "complete") {
        sovereignHash = update.txHash;
      }
    };
    await Promise.allSettled([
      launch(id, { ...base, route: "LegacySwift" }, sink),
      launch(id, { ...base, route: "SovereignRs" }, sink),
    ]);
    if (sovereignHash) {
      remember({
        id,
        date: new Date().toLocaleString("en-GB", { dateStyle: "medium", timeStyle: "short" }),
        type: "Swap",
        amount: money(amount),
        currency: "BRL-CBDC",
        status: "Settled",
        tx_hash: sovereignHash,
        verified: true,
      });
    }
    setRunning(false);
  }

  const received = amount * 1.2;
  const legacyDays = "~2–3 days";
  const sovereignSeconds =
    sovereign.totalMs != null ? `${(sovereign.totalMs / 1000).toFixed(1)}s` : "< 2s (demo)";

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="grid min-h-0 flex-1 grid-cols-[320px_1fr] gap-4">
        <form
          className="flex flex-col gap-3 rounded-xl border border-slate-800 bg-slate-950/80 p-4 shadow-xl shadow-black/20"
          onSubmit={(event) => {
            event.preventDefault();
            void send();
          }}
        >
          <div className="flex items-center gap-2 text-emerald-400">
            <Zap className="h-4 w-4" />
            <span className="text-sm font-semibold tracking-wide text-slate-100">Dual-track settlement</span>
          </div>
          <p className="text-[11px] leading-relaxed text-slate-400">
            One instruction, two rails. Steps stream from the Rust backend over Tauri events. A restricted desk is refused before any CBDC moves.
          </p>
          <label className="text-[11px] text-slate-400">
            From
            <select
              className="mt-1 w-full rounded-md border border-slate-800 bg-slate-950 px-2 py-2 text-sm text-slate-100"
              value={from}
              onChange={(event) => setFrom(event.target.value)}
            >
              {INSTITUTIONS.map((name) => (
                <option key={name}>{name}</option>
              ))}
            </select>
          </label>
          <label className="text-[11px] text-slate-400">
            To
            <select
              className="mt-1 w-full rounded-md border border-slate-800 bg-slate-950 px-2 py-2 text-sm text-slate-100"
              value={to}
              onChange={(event) => setTo(event.target.value)}
            >
              {INSTITUTIONS.map((name) => (
                <option key={name}>{name}</option>
              ))}
            </select>
          </label>
          <label className="text-[11px] text-slate-400">
            Amount (BRL-CBDC)
            <input
              className="mt-1 w-full rounded-md border border-slate-800 bg-slate-950 px-2 py-2 font-mono text-sm text-slate-100"
              type="number"
              min={1}
              value={amount}
              onChange={(event) => setAmount(Number(event.target.value))}
            />
          </label>
          <div className="rounded-lg border border-slate-800 bg-slate-950 px-3 py-2 text-xs">
            <div className="flex items-center justify-between text-slate-300">
              <span>BRL</span>
              <motion.span layout className="font-mono text-blue-300">
                {money(amount)}
              </motion.span>
            </div>
            <div className="my-1 flex items-center justify-center gap-1 text-[10px] uppercase tracking-widest text-slate-500">
              <ArrowRight className="h-3 w-3" /> oracle 1.20
            </div>
            <div className="flex items-center justify-between text-slate-300">
              <span>CNY</span>
              <span className="font-mono text-emerald-300">{money(received)}</span>
            </div>
          </div>
          <Button type="submit" disabled={running || amount <= 0 || from === to} className="w-full">
            {running ? "Processing…" : "Execute on both rails"}
          </Button>
        </form>
        <div className="grid min-h-0 grid-cols-2 gap-4">
          <SettlementPipeline
            title="Legacy · SWIFT"
            subtitle="Correspondent friction & USD clearing"
            route="LegacySwift"
            track={legacy}
          />
          <SettlementPipeline
            title="Sovereign · Swift-RS"
            subtitle="BFT finality with ZK compliance"
            route="SovereignRs"
            track={sovereign}
          />
        </div>
      </div>
      <div className="grid grid-cols-3 gap-3 rounded-xl border border-slate-800 bg-slate-950/60 px-4 py-3 text-xs">
        <div>
          <div className="text-slate-500">Legacy time</div>
          <div className="font-mono text-lg text-amber-300">
            {legacy.totalMs ? `${(legacy.totalMs / 1000).toFixed(1)}s demo` : legacyDays}
          </div>
        </div>
        <div>
          <div className="text-slate-500">Sovereign time</div>
          <div className="font-mono text-lg text-emerald-300">{sovereignSeconds}</div>
        </div>
        <div>
          <div className="text-slate-500">Fee delta</div>
          <div className="font-mono text-lg text-slate-100">
            {(legacy.feeUsd ?? 45).toFixed(2)} → {(sovereign.feeUsd ?? 0.01).toFixed(2)} USD
          </div>
        </div>
      </div>
    </div>
  );
}
