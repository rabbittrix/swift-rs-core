import { useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { mockPayment } from "@/lib/api";
import type { PaymentStatus } from "@/lib/types";

const PARTIES = ["Banco do Brasil", "Bank of China", "Restricted Desk"];

const FLAGS = [
  { id: "aml-1", title: "Corridor velocity", score: "0.12", result: "Cleared", tone: "emerald" as const, note: "Volume sits inside the daily limit for this desk." },
  { id: "aml-2", title: "Purpose review", score: "0.64", result: "Held", tone: "amber" as const, note: "AI score kept the payment for a purpose check. It is not auto-released." },
  { id: "aml-3", title: "Restricted-list match", score: "0.98", result: "Blocked", tone: "rose" as const, note: "Name match stops the payment. There is no override on this desk." },
];

export function CompliancePanel() {
  const [party, setParty] = useState(PARTIES[0]);
  const [outcome, setOutcome] = useState<PaymentStatus | null>(null);
  const clear = outcome?.status === "settled";

  return (
    <div className="grid h-full grid-cols-2 gap-4">
      <Card>
        <CardHeader>
          <CardTitle>ZK compliance check</CardTitle>
          <Badge tone={outcome ? (clear ? "emerald" : "rose") : "slate"}>{outcome ? (clear ? "verified" : "refused") : "idle"}</Badge>
        </CardHeader>
        <CardContent className="space-y-3 text-sm">
          <p className="text-xs leading-relaxed text-slate-400">
            The public network sees a limit, a sanctions-clear flag, and a payment binding. It does not see the amount or the full identity. A listed party is rejected before a proof is built.
          </p>
          <label className="block text-[11px] text-slate-400">
            Party
            <select className="mt-1 w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-2 text-sm text-slate-100" value={party} onChange={(event) => { setParty(event.target.value); setOutcome(null); }}>
              {PARTIES.map((name) => (
                <option key={name}>{name}</option>
              ))}
            </select>
          </label>
          <Button
            type="button"
            onClick={() =>
              setOutcome(
                mockPayment({
                  from: party,
                  to: "Bank of China",
                  amount: 1_000_000,
                  currency_from: "BRL-CBDC",
                  currency_to: "CNY-CBDC",
                  route: "SOVEREIGN",
                }),
              )
            }
          >
            Run check
          </Button>
          {outcome && (
            <dl className="grid grid-cols-2 gap-2 rounded-lg border border-slate-800 p-3 text-xs">
              <dt className="text-slate-500">Public limit</dt>
              <dd className="text-right font-mono">100</dd>
              <dt className="text-slate-500">sanctions_clear</dt>
              <dd className="text-right font-mono">{clear ? "1" : "refused"}</dd>
              <dt className="text-slate-500">Amount</dt>
              <dd className="text-right">hidden</dd>
              <dt className="text-slate-500">Identity</dt>
              <dd className="text-right">hidden</dd>
              <dt className="text-slate-500">Binding</dt>
              <dd className="break-all text-right font-mono text-[11px] text-slate-300">{outcome.tx_hash ?? "no proof"}</dd>
            </dl>
          )}
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>AML queue</CardTitle>
          <Badge tone="blue">swift-rs-ai scores</Badge>
        </CardHeader>
        <CardContent className="space-y-3">
          {FLAGS.map((flag) => (
            <div key={flag.id} className="rounded-lg border border-slate-800 px-3 py-3">
              <div className="flex items-center justify-between">
                <div className="text-sm text-slate-100">{flag.title}</div>
                <Badge tone={flag.tone}>{flag.result}</Badge>
              </div>
              <div className="mt-1 font-mono text-[11px] text-slate-500">score {flag.score}</div>
              <p className="mt-2 text-xs text-slate-400">{flag.note}</p>
            </div>
          ))}
        </CardContent>
      </Card>
    </div>
  );
}
