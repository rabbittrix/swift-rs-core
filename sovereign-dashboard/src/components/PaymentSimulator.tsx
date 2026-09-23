import { useCallback, useEffect, useRef, useState } from "react";
import { motion } from "framer-motion";
import { ArrowRight, Zap } from "lucide-react";
import { CountrySelector } from "@/components/CountrySelector";
import { GeopoliticalRiskBadge } from "@/components/GeopoliticalRiskBadge";
import { Button } from "@/components/ui/button";
import { SettlementPipeline } from "@/components/SettlementPipeline";
import {
  getAvailableCountries,
  isTauri,
  runSimulatePayment,
  subscribeTransactionUpdates,
} from "@/lib/api";
import { corridorContext, legacyFee } from "@/lib/countries";
import { applyUpdate, cbdcPair, emptyTrack, simulatePaymentBrowser } from "@/lib/simulation";
import type { CountryNode, TrackSnapshot, TransactionUpdate } from "@/lib/types";
import { money } from "@/lib/utils";
import { useDesk } from "@/store/useDesk";

export function PaymentSimulator() {
  const remember = useDesk((state) => state.remember);
  const [countries, setCountries] = useState<CountryNode[]>([]);
  const [origin, setOrigin] = useState<CountryNode | null>(null);
  const [dest, setDest] = useState<CountryNode | null>(null);
  const [amount, setAmount] = useState(10_000_000);
  const [running, setRunning] = useState(false);
  const [legacy, setLegacy] = useState<TrackSnapshot>(emptyTrack());
  const [sovereign, setSovereign] = useState<TrackSnapshot>(emptyTrack());
  const activePayment = useRef<string | null>(null);

  useEffect(() => {
    void getAvailableCountries().then((list) => {
      setCountries(list);
      setOrigin(list.find((item) => item.id === "br") ?? list[0] ?? null);
      setDest(list.find((item) => item.id === "cn") ?? list[1] ?? null);
    });
  }, []);

  const ctx = origin && dest ? corridorContext(origin.id, dest.id) : null;

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

  async function launch(
    id: string,
    originId: string,
    destId: string,
    route: "LegacySwift" | "SovereignRs",
    sink: (u: TransactionUpdate) => void,
  ) {
    const request = { paymentId: id, originCountryId: originId, destinationCountryId: destId, amount, route };
    if (isTauri()) {
      await runSimulatePayment(request);
      return;
    }
    await simulatePaymentBrowser(request, sink);
  }

  async function send() {
    if (!origin || !dest || origin.id === dest.id) return;
    setRunning(true);
    const id = crypto.randomUUID();
    activePayment.current = id;
    setLegacy({ ...emptyTrack(), phase: "running" });
    setSovereign({ ...emptyTrack(), phase: "running" });
    let sovereignHash: string | null = null;
    const sink = (update: TransactionUpdate) => {
      dispatch(update);
      if (update.paymentId === id && update.route === "SovereignRs" && update.kind === "complete") {
        sovereignHash = update.txHash;
      }
    };
    await Promise.allSettled([
      launch(id, origin.id, dest.id, "LegacySwift", sink),
      launch(id, origin.id, dest.id, "SovereignRs", sink),
    ]);
    if (sovereignHash) {
      remember({
        id,
        date: new Date().toLocaleString("en-GB", { dateStyle: "medium", timeStyle: "short" }),
        type: "Swap",
        amount: money(amount),
        currency: `${origin.fiatCurrency}-CBDC`,
        status: "Settled",
        tx_hash: sovereignHash,
        verified: true,
      });
    }
    setRunning(false);
  }

  const pair = origin && dest ? cbdcPair(origin.id, dest.id) : { from: "—", to: "—" };
  const oracleHint =
    origin?.fiatCurrency === "BRL" && dest?.fiatCurrency === "CNY"
      ? "1.20"
      : origin?.fiatCurrency === "USD"
        ? "FX desk"
        : "oracle";

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="grid min-h-0 flex-1 grid-cols-[340px_1fr] gap-4">
        <form
          className="flex flex-col gap-3 overflow-auto rounded-xl border border-slate-800 bg-slate-950/80 p-4 shadow-xl shadow-black/20"
          onSubmit={(event) => {
            event.preventDefault();
            void send();
          }}
        >
          <div className="flex items-center gap-2 text-emerald-400">
            <Zap className="h-4 w-4" />
            <span className="text-sm font-semibold tracking-wide text-slate-100">Global corridor</span>
          </div>
          {countries.length > 0 && origin && dest && (
            <>
              <CountrySelector
                label="Origin country / central bank"
                countries={countries}
                valueId={origin.id}
                onChange={setOrigin}
              />
              <CountrySelector
                label="Destination country / central bank"
                countries={countries}
                valueId={dest.id}
                onChange={setDest}
              />
            </>
          )}
          <GeopoliticalRiskBadge ctx={ctx} />
          <label className="text-[11px] text-slate-400">
            Amount ({origin?.fiatCurrency ?? "—"})
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
              <span>{pair.from}</span>
              <motion.span layout className="font-mono text-blue-300">
                {money(amount)}
              </motion.span>
            </div>
            <div className="my-1 flex items-center justify-center gap-1 text-[10px] uppercase tracking-widest text-slate-500">
              <ArrowRight className="h-3 w-3" /> {oracleHint}
            </div>
            <div className="flex items-center justify-between text-slate-300">
              <span>{pair.to}</span>
              <span className="font-mono text-emerald-300">{dest?.fiatCurrency ?? "—"} leg</span>
            </div>
          </div>
          <Button type="submit" disabled={running || amount <= 0 || !origin || !dest || origin.id === dest.id}>
            {running ? "Processing…" : "Simulate both rails"}
          </Button>
        </form>
        <div className="grid min-h-0 grid-cols-2 gap-4">
          <SettlementPipeline
            title="Legacy · SWIFT"
            subtitle={ctx?.highSwiftFriction ? "OFAC / USD friction" : "Correspondent chain"}
            route="LegacySwift"
            track={legacy}
          />
          <SettlementPipeline
            title="Sovereign · Swift-RS"
            subtitle={ctx?.sanctionedTouch ? "Fail-closed compliance" : "Direct CBDC corridor"}
            route="SovereignRs"
            track={sovereign}
          />
        </div>
      </div>
      <div className="grid grid-cols-3 gap-3 rounded-xl border border-slate-800 bg-slate-950/60 px-4 py-3 text-xs">
        <div>
          <div className="text-slate-500">Legacy fee (est.)</div>
          <div className="font-mono text-lg text-amber-300">
            ${ctx ? legacyFee(ctx).toFixed(2) : "45–150"}
          </div>
        </div>
        <div>
          <div className="text-slate-500">Sovereign time</div>
          <div className="font-mono text-lg text-emerald-300">
            {sovereign.totalMs ? `${(sovereign.totalMs / 1000).toFixed(1)}s` : "< 2s demo"}
          </div>
        </div>
        <div>
          <div className="text-slate-500">Corridor</div>
          <div className="font-mono text-sm text-slate-100">
            {origin?.flag} {origin?.fiatCurrency} → {dest?.flag} {dest?.fiatCurrency}
          </div>
        </div>
      </div>
    </div>
  );
}
