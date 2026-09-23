import { AnimatePresence, motion } from "framer-motion";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { RouteType, TrackSnapshot } from "@/lib/types";
import { cn } from "@/lib/utils";

const toneRing = {
  LegacySwift: "ring-amber-500/25",
  SovereignRs: "ring-emerald-500/30",
};

const dot = {
  active: "bg-blue-400 shadow-[0_0_12px_#60a5fa]",
  done: "bg-emerald-400",
  blocked: "bg-rose-400",
  amber: "bg-amber-400",
  emerald: "bg-emerald-400",
  blue: "bg-blue-400",
  rose: "bg-rose-400",
};

export function SettlementPipeline({
  title,
  subtitle,
  route,
  track,
}: {
  title: string;
  subtitle: string;
  route: RouteType;
  track: TrackSnapshot;
}) {
  const sovereign = route === "SovereignRs";
  const phaseTone =
    track.phase === "blocked" ? "rose" : track.phase === "settled" ? "emerald" : sovereign ? "blue" : "amber";

  return (
    <Card className={cn("flex h-full flex-col ring-1", toneRing[route])}>
      <CardHeader>
        <div>
          <CardTitle>{title}</CardTitle>
          <p className="mt-0.5 text-[11px] text-slate-500">{subtitle}</p>
        </div>
        <Badge tone={phaseTone}>{track.phase === "idle" ? "ready" : track.phase}</Badge>
      </CardHeader>
      <CardContent className="flex min-h-0 flex-1 flex-col gap-3">
        <div className="grid grid-cols-2 gap-2 text-xs">
          <div className={cn("rounded-lg px-3 py-2", sovereign ? "bg-emerald-500/10" : "bg-amber-500/10")}>
            <div className="text-slate-500">Fee</div>
            <div className={cn("font-mono text-base", sovereign ? "text-emerald-300" : "text-amber-300")}>
              {track.feeUsd != null ? `$${track.feeUsd.toFixed(2)}` : sovereign ? "$0.01" : "$45.00"}
            </div>
          </div>
          <div className="rounded-lg bg-slate-950/80 px-3 py-2">
            <div className="text-slate-500">Elapsed</div>
            <div className="font-mono text-base text-slate-100">
              {track.totalMs != null ? `${(track.totalMs / 1000).toFixed(1)}s` : track.phase === "running" ? "…" : "—"}
            </div>
          </div>
        </div>
        <ul className="min-h-0 flex-1 space-y-2 overflow-auto pr-1">
          <AnimatePresence initial={false}>
            {track.steps.map((step) => (
              <motion.li
                key={`${step.index}-${step.status}`}
                initial={{ opacity: 0, x: sovereign ? 12 : -12 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0 }}
                transition={{ duration: 0.22 }}
                className="flex gap-3 rounded-lg border border-slate-800/90 bg-slate-950/40 px-3 py-2"
              >
                <span
                  className={cn(
                    "mt-1 h-2.5 w-2.5 shrink-0 rounded-full",
                    step.status === "active" ? dot.active : dot[step.tone],
                    step.status === "active" && "animate-pulse",
                  )}
                />
                <div>
                  <div className="text-xs font-medium text-slate-100">{step.label}</div>
                  <div className="text-[11px] text-slate-500">{step.detail}</div>
                </div>
              </motion.li>
            ))}
          </AnimatePresence>
          {track.phase === "idle" && (
            <li className="rounded-lg border border-dashed border-slate-800 px-3 py-6 text-center text-xs text-slate-500">
              Awaiting instruction…
            </li>
          )}
        </ul>
        {track.txHash && track.phase === "settled" && (
          <motion.div
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            className="rounded-lg border border-emerald-500/30 bg-emerald-500/10 p-3"
          >
            <div className="text-[11px] uppercase tracking-wider text-emerald-300">Verified binding</div>
            <div className="mt-1 break-all font-mono text-[11px] text-emerald-100">{track.txHash}</div>
          </motion.div>
        )}
      </CardContent>
    </Card>
  );
}
