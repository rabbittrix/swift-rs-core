import { useEffect, useState } from "react";
import { Area, AreaChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { getGlobalNetworkSnapshot } from "@/lib/api";
import type { GlobalNetworkSnapshot } from "@/lib/types";
import { useDesk } from "@/store/useDesk";

export function NetworkBoard() {
  const stats = useDesk((state) => state.stats);
  const series = useDesk((state) => state.tpsSeries);
  const feed = useDesk((state) => state.feed);
  const validators = stats?.validators ?? [];
  const [global, setGlobal] = useState<GlobalNetworkSnapshot | null>(null);

  useEffect(() => {
    void getGlobalNetworkSnapshot().then(setGlobal);
  }, []);

  return (
    <div className="grid h-full grid-cols-[1.35fr_0.85fr] gap-4">
      <div className="flex min-h-0 flex-col gap-4">
        <Card className="relative min-h-[260px] flex-1 overflow-hidden border-slate-800 bg-slate-950/80">
          <CardHeader>
            <CardTitle>Global node map</CardTitle>
            <Badge tone="blue">10 jurisdictions</Badge>
          </CardHeader>
          <CardContent className="relative h-[calc(100%-3.5rem)]">
            <div className="absolute inset-4 rounded-xl border border-dashed border-slate-800 bg-[radial-gradient(circle_at_center,rgba(30,58,138,0.22),transparent_60%)]" />
            {validators.map((node) => {
              const hot = feed.some((tx) => tx.from_node.includes(node.city) || tx.to_node.includes(node.city));
              return (
                <div key={node.id} className="absolute" style={{ left: `${node.x}%`, top: `${node.y}%` }}>
                  <div
                    className={`h-2.5 w-2.5 -translate-x-1/2 rounded-full ${hot ? "bg-emerald-400 shadow-[0_0_14px_#34d399]" : "bg-blue-400"}`}
                  />
                  <div className="mt-0.5 -translate-x-1/2 max-w-[120px] truncate text-center text-[10px] font-medium text-slate-200">
                    {node.city}
                  </div>
                </div>
              );
            })}
          </CardContent>
        </Card>
        <Card className="border-slate-800 bg-slate-950/80">
          <CardHeader>
            <CardTitle>Transactions per second</CardTitle>
            <span className="font-mono text-xs text-slate-400">simulated corridor</span>
          </CardHeader>
          <CardContent className="h-32">
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
        <Card className="border-slate-800 bg-slate-950/80">
          <CardHeader>
            <CardTitle>By region</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {global?.regions.map((row) => (
              <div key={row.region} className="flex items-center justify-between rounded-lg border border-slate-800 px-3 py-2 text-xs">
                <span className="text-slate-200">{row.label}</span>
                <span className="font-mono text-emerald-300">{row.activeNodes} nós ativos</span>
              </div>
            ))}
          </CardContent>
        </Card>
        <Card className="border-slate-800 bg-slate-950/80">
          <CardHeader>
            <CardTitle>Cross-border volume</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {global?.corridors.map((row) => (
              <div key={row.pair} className="flex items-center justify-between rounded-lg border border-slate-800 px-3 py-2 text-xs">
                <span className="font-mono text-slate-200">{row.pair}</span>
                <span className="text-emerald-300">{row.volumeUsd}</span>
              </div>
            ))}
          </CardContent>
        </Card>
        <Card className="min-h-0 flex-1 border-slate-800 bg-slate-950/80">
          <CardHeader>
            <CardTitle>Live corridor</CardTitle>
            <Badge tone="emerald">streaming</Badge>
          </CardHeader>
          <CardContent className="space-y-2 overflow-auto">
            {feed.length === 0 && <p className="text-xs text-slate-500">Waiting for the next settlement…</p>}
            {feed.map((tx) => (
              <div key={tx.id} className="rounded-lg border border-slate-800 px-3 py-2 text-xs">
                <div className="flex items-center justify-between text-slate-200">
                  <span className="truncate pr-2">{tx.from_node}</span>
                  <span className="text-slate-500">→</span>
                  <span className="truncate pl-2 text-right">{tx.to_node}</span>
                </div>
                <div className="mt-1 flex justify-between font-mono text-[11px] text-slate-400">
                  <span>
                    {tx.amount} {tx.currency}
                  </span>
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
