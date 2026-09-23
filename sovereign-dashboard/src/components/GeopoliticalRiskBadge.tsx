import { Badge } from "@/components/ui/badge";
import { riskLabels, type CorridorContext } from "@/lib/countries";

export function GeopoliticalRiskBadge({ ctx }: { ctx: CorridorContext | null }) {
  const labels = riskLabels(ctx);
  return (
    <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
      <div className="rounded-lg border border-slate-800 bg-slate-950/60 px-3 py-2">
        <div className="text-[10px] uppercase tracking-wider text-slate-500">SWIFT</div>
        <Badge tone={labels.swift.tone} className="mt-1 whitespace-normal text-left leading-snug">
          {labels.swift.text}
        </Badge>
      </div>
      <div className="rounded-lg border border-slate-800 bg-slate-950/60 px-3 py-2">
        <div className="text-[10px] uppercase tracking-wider text-slate-500">Sovereign</div>
        <Badge tone={labels.sovereign.tone} className="mt-1 whitespace-normal text-left leading-snug">
          {labels.sovereign.text}
        </Badge>
      </div>
    </div>
  );
}
