import { motion } from "framer-motion";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { PaymentState, RouteKind, TrackStep } from "@/lib/types";
import { cn } from "@/lib/utils";

const tone = {
  idle: "bg-slate-700",
  active: "bg-amber-400",
  done: "bg-emerald-400",
  blocked: "bg-rose-400",
};

export function TrackCard({
  kind,
  phase,
  steps,
  fee,
  time,
  hash,
  gate,
}: {
  kind: RouteKind;
  phase: PaymentState;
  steps: TrackStep[];
  fee: string;
  time: string;
  hash: string | null;
  gate: "clear" | "blocked" | "pending" | null;
}) {
  const sovereign = kind === "SOVEREIGN";
  return (
    <Card className={cn("flex h-full flex-col", sovereign ? "ring-1 ring-blue-500/30" : "ring-1 ring-amber-500/20")}>
      <CardHeader>
        <CardTitle>{sovereign ? "Sovereign · Swift-RS" : "Legacy · SWIFT"}</CardTitle>
        <Badge tone={phase === "blocked" ? "rose" : phase === "settled" ? "emerald" : sovereign ? "blue" : "amber"}>
          {phase === "idle" ? "ready" : phase}
        </Badge>
      </CardHeader>
      <CardContent className="flex min-h-0 flex-1 flex-col gap-3">
        <div className="grid grid-cols-2 gap-2 text-xs">
          <div className="rounded-lg bg-slate-950/60 px-3 py-2">
            <div className="text-slate-500">Fee</div>
            <div className={cn("font-mono text-base", sovereign ? "text-emerald-300" : "text-amber-300")}>{fee}</div>
          </div>
          <div className="rounded-lg bg-slate-950/60 px-3 py-2">
            <div className="text-slate-500">Settlement</div>
            <div className="font-mono text-base text-slate-100">{time}</div>
          </div>
        </div>
        {gate && (
          <div
            className={cn(
              "rounded-md px-3 py-2 text-xs",
              gate === "blocked" ? "bg-rose-500/10 text-rose-200" : gate === "clear" ? "bg-emerald-500/10 text-emerald-200" : "bg-slate-800 text-slate-300",
            )}
          >
            {gate === "blocked"
              ? "Sanctions screen refused the counterparty. No proof is built and nothing is settled."
              : gate === "clear"
                ? "Sanctions screen clear. Amount stays off the public explorer."
                : "Sanctions screen runs before any lock."}
          </div>
        )}
        <ol className="space-y-2">
          {steps.map((step) => (
            <li key={step.label} className="flex gap-3 rounded-lg border border-slate-800/80 px-3 py-2">
              <span className={cn("mt-1 h-2.5 w-2.5 shrink-0 rounded-full", tone[step.state], step.state === "active" && "animate-pulse")} />
              <div>
                <div className="text-xs font-medium text-slate-100">{step.label}</div>
                <div className="text-[11px] text-slate-500">{step.detail}</div>
              </div>
            </li>
          ))}
        </ol>
        {hash && phase === "settled" && (
          <motion.div
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            className="mt-auto rounded-lg border border-emerald-500/30 bg-emerald-500/10 p-3"
          >
            <div className="text-[11px] uppercase tracking-wider text-emerald-300">Settled · proof binding</div>
            <div className="mt-1 break-all font-mono text-[11px] text-emerald-100">{hash}</div>
          </motion.div>
        )}
      </CardContent>
    </Card>
  );
}
