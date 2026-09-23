import { Area, AreaChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useDesk } from "@/store/useDesk";

export function NetworkBoard() {
  const stats = useDesk((state) => state.stats);
  const series = useDesk((state) => state.tpsSeries);
  const feed = useDesk((state) => state.feed);
  const validators = stats?.validators ?? [];

  return (
    <div className="grid h-full grid-cols-[1.4fr_0.8fr] gap-4">
      <div className="flex min-h-0 flex-col gap-4">
        <Card className="relative min-h-[280px] flex-1 overflow-hidden">
          <CardHeader>
            <CardTitle>Settlement validators</CardTitle>
            <Badge tone="blue">local pilot</Badge>
          </CardHeader>
          <CardContent className="relative h-[calc(100%-3.5rem)]">
            <div className="absolute inset-4 rounded-xl border border-dashed border-slate-800 bg-[radial-gradient(circle_at_center,rgba(30,58,138,0.25),transparent_60%)]" />
            {validators.map((node) => {
              const hot = feed.some((tx) => tx.from_node === node.name || tx.to_node === node.name);
              return (
                <div key={node.id} className="absolute" style={{ left: `${node.x}%`, top: `${node.y}%` }}>
                  <div className={`h-3 w-3 -translate-x-1/2 rounded-full ${hot ? "bg-emerald-400 shadow-[0_0_16px_#34d399]" : "bg-blue-400"}`} />
                  <div className="mt-1 -translate-x-1/2 whitespace-nowrap text-[11px]">
                    <div className="font-medium text-slate-100">{node.name}</div>
                    <div className="text-slate-500">{node.city}</div>
                  </div>
                </div>
              );
            })}
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Transactions per second</CardTitle>
            <span className="font-mono text-xs text-slate-400">simulated corridor</span>
          </CardHeader>
          <CardContent className="h-36">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={series}>
                <XAxis dataKey="t" hide />
                <YAxis hide domain={[4000, 5000]} />
                <Tooltip
                  contentStyle={{ background: "#0f172a", border: "1px solid #1e293b", borderRadius: 8, fontSize: 12 }}
                />
                <Area type="monotone" dataKey="tps" stroke="#60a5fa" fill="#1d4ed8" fillOpacity={0.35} />
              </AreaChart>
            </ResponsiveContainer>
          </CardContent>
        </Card>
      </div>
      <div className="flex min-h-0 flex-col gap-4">
        <div className="grid grid-cols-1 gap-3">
          <Metric label="TPS" value={stats ? stats.tps.toLocaleString() : "—"} />
          <Metric label="Volume settled today" value={stats?.total_volume_24h ?? "—"} />
          <Metric label="Average settlement" value={stats ? `${stats.avg_settlement_ms} ms` : "—"} />
          <Metric label="Corridor institutions" value={stats ? String(stats.active_nodes) : "—"} hint="5 of these are settlement validators" />
        </div>
        <Card className="min-h-0 flex-1">
          <CardHeader>
            <CardTitle>Live corridor</CardTitle>
            <Badge tone="emerald">streaming</Badge>
          </CardHeader>
          <CardContent className="space-y-2 overflow-auto">
            {feed.length === 0 && <p className="text-xs text-slate-500">Waiting for the next settlement…</p>}
            {feed.map((tx) => (
              <div key={tx.id} className="rounded-lg border border-slate-800 px-3 py-2 text-xs">
                <div className="flex items-center justify-between text-slate-200">
                  <span>{tx.from_node}</span>
                  <span className="text-slate-500">→</span>
                  <span>{tx.to_node}</span>
                </div>
                <div className="mt-1 flex justify-between font-mono text-[11px] text-slate-400">
                  <span>{tx.amount} {tx.currency}</span>
                  <span>{tx.settled_ms} ms</span>
                </div>
              </div>
            ))}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

function Metric({ label, value, hint }: { label: string; value: string; hint?: string }) {
  return (
    <div className="rounded-xl border border-desk-line bg-desk-panel/90 px-4 py-3">
      <div className="text-[11px] uppercase tracking-wider text-slate-500">{label}</div>
      <div className="font-mono text-xl text-slate-50">{value}</div>
      {hint && <div className="text-[11px] text-slate-500">{hint}</div>}
    </div>
  );
}
