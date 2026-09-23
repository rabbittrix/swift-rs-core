import { useState } from "react";
import { motion } from "framer-motion";
import { Button } from "@/components/ui/button";
import { TrackCard } from "@/components/TrackCard";
import { initiatePayment } from "@/lib/api";
import type { PaymentState, TrackStep } from "@/lib/types";
import { money, wait } from "@/lib/utils";
import { useDesk } from "@/store/useDesk";

const INSTITUTIONS = [
  "Banco do Brasil",
  "Bank of China",
  "First Abu Dhabi Bank",
  "State Bank of India",
  "Deutsche Bundesbank desk",
  "Restricted Desk",
];

const LEGACY: Array<[string, string]> = [
  ["Sending to correspondent bank", "Instruction handed to the nostro agent"],
  ["Waiting for USD clearing (NY)", "Payment parked in the dollar correspondent chain"],
  ["OFAC compliance check", "Name screen pending at the correspondent"],
  ["Final settlement", "Beneficiary credit after the chain completes"],
];

const SOVEREIGN: Array<[string, string]> = [
  ["Lock BRL-CBDC", "Source units locked by the Brazilian issuer"],
  ["Atomic swap via smart contract", "Both legs settle together or not at all"],
  ["Mint CNY-CBDC", "Destination units issued against the locked reais"],
  ["Final settlement", "Consortium certificate and public proof binding"],
];

function idleSteps(rows: Array<[string, string]>): TrackStep[] {
  return rows.map(([label, detail]) => ({ label, detail, state: "idle" }));
}

export function Simulator() {
  const remember = useDesk((state) => state.remember);
  const [from, setFrom] = useState(INSTITUTIONS[0]);
  const [to, setTo] = useState(INSTITUTIONS[1]);
  const [amount, setAmount] = useState(5_000_000);
  const [running, setRunning] = useState(false);
  const [legacyPhase, setLegacyPhase] = useState<PaymentState>("idle");
  const [sovereignPhase, setSovereignPhase] = useState<PaymentState>("idle");
  const [legacySteps, setLegacySteps] = useState(idleSteps(LEGACY));
  const [sovereignSteps, setSovereignSteps] = useState(idleSteps(SOVEREIGN));
  const [gate, setGate] = useState<"clear" | "blocked" | "pending" | null>(null);
  const [hash, setHash] = useState<string | null>(null);

  async function play(
    rows: Array<[string, string]>,
    setSteps: (steps: TrackStep[]) => void,
    setPhase: (phase: PaymentState) => void,
    blockedAt: number | null,
    pace: number,
  ) {
    const steps = idleSteps(rows);
    setPhase("running");
    for (let index = 0; index < steps.length; index += 1) {
      steps[index] = { ...steps[index], state: "active" };
      setSteps([...steps]);
      await wait(pace);
      const blocked = blockedAt === index;
      steps[index] = { ...steps[index], state: blocked ? "blocked" : "done" };
      setSteps([...steps]);
      if (blocked) {
        setPhase("blocked");
        return;
      }
    }
    setPhase("settled");
  }

  async function send() {
    setRunning(true);
    setHash(null);
    setGate("pending");
    setLegacySteps(idleSteps(LEGACY));
    setSovereignSteps(idleSteps(SOVEREIGN));
    const request = {
      from,
      to,
      amount,
      currency_from: "BRL-CBDC",
      currency_to: "CNY-CBDC",
    };
    const [legacy, sovereign] = await Promise.all([
      initiatePayment({ ...request, route: "LEGACY" }),
      initiatePayment({ ...request, route: "SOVEREIGN" }),
    ]);
    setGate(sovereign.status === "blocked" ? "blocked" : "clear");
    if (sovereign.status === "settled" && sovereign.tx_hash) setHash(sovereign.tx_hash);
    await Promise.all([
      play(LEGACY, setLegacySteps, setLegacyPhase, legacy.status === "blocked" ? 2 : null, 1400),
      sovereign.status === "blocked"
        ? Promise.resolve(setSovereignPhase("blocked"))
        : play(SOVEREIGN, setSovereignSteps, setSovereignPhase, null, 380),
    ]);
    if (sovereign.status === "settled" && sovereign.tx_hash) {
      remember({
        id: sovereign.id,
        date: "23 Sep 2026",
        type: "Swap",
        amount: money(amount),
        currency: "BRL-CBDC",
        status: "Settled",
        tx_hash: sovereign.tx_hash,
      });
    }
    setRunning(false);
  }

  const received = amount * 1.2;

  return (
    <div className="grid h-full grid-cols-[300px_1fr] gap-4">
      <form
        className="flex flex-col gap-3 rounded-xl border border-desk-line bg-desk-panel/90 p-4"
        onSubmit={(event) => {
          event.preventDefault();
          void send();
        }}
      >
        <div>
          <div className="text-sm font-semibold">Cross-border payment</div>
          <p className="mt-1 text-[11px] leading-relaxed text-slate-400">
            Both rails run from the same instruction. A restricted desk is refused before any CBDC moves.
          </p>
        </div>
        <label className="text-[11px] text-slate-400">
          From
          <select className="mt-1 w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-2 text-sm" value={from} onChange={(event) => setFrom(event.target.value)}>
            {INSTITUTIONS.map((name) => (
              <option key={name}>{name}</option>
            ))}
          </select>
        </label>
        <label className="text-[11px] text-slate-400">
          To
          <select className="mt-1 w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-2 text-sm" value={to} onChange={(event) => setTo(event.target.value)}>
            {INSTITUTIONS.map((name) => (
              <option key={name}>{name}</option>
            ))}
          </select>
        </label>
        <label className="text-[11px] text-slate-400">
          Amount (BRL-CBDC)
          <input
            className="mt-1 w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-2 font-mono text-sm"
            type="number"
            min={1}
            value={amount}
            onChange={(event) => setAmount(Number(event.target.value))}
          />
        </label>
        <div className="rounded-lg bg-slate-950/70 px-3 py-2 text-xs text-slate-300">
          <div className="flex items-center justify-between">
            <span>BRL</span>
            <motion.span layout className="font-mono text-blue-300">{money(amount)}</motion.span>
          </div>
          <div className="my-1 text-center text-[10px] uppercase tracking-widest text-slate-500">oracle 1.20</div>
          <div className="flex items-center justify-between">
            <span>CNY</span>
            <span className="font-mono text-emerald-300">{money(received)}</span>
          </div>
        </div>
        <Button type="submit" disabled={running || amount <= 0 || from === to}>
          {running ? "Settling…" : "Send on both rails"}
        </Button>
        <p className="text-[10px] leading-relaxed text-slate-500">
          Legacy timing is a time-lapse of a 2–3 day chain. Sovereign steps finish in under two seconds.
        </p>
      </form>
      <div className="grid min-h-0 grid-cols-2 gap-4">
        <TrackCard kind="LEGACY" phase={legacyPhase} steps={legacySteps} fee="$45.00" time="2–3 days" hash={null} gate={null} />
        <TrackCard kind="SOVEREIGN" phase={sovereignPhase} steps={sovereignSteps} fee="$0.01" time="< 2 seconds" hash={hash} gate={gate} />
      </div>
    </div>
  );
}
