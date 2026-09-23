import { useState } from "react";
import { BadgeCheck, ExternalLink } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useDesk } from "@/store/useDesk";

export function WalletView() {
  const balances = useDesk((state) => state.balances);
  const history = useDesk((state) => state.history);
  const [openHash, setOpenHash] = useState<string | null>(null);

  return (
    <div className="grid h-full grid-cols-[0.95fr_1.25fr] gap-4">
      <div className="grid content-start gap-3">
        {balances.map((balance) => (
          <Card key={balance.currency} className="border-slate-800 bg-slate-950/80">
            <CardHeader>
              <CardTitle>{balance.currency}</CardTitle>
              <Badge tone="blue">{balance.name}</Badge>
            </CardHeader>
            <CardContent>
              <div className="font-mono text-2xl text-slate-50">{balance.amount}</div>
              <div className="mt-1 text-[11px] text-slate-500">Issuer · {balance.issuer}</div>
            </CardContent>
          </Card>
        ))}
      </div>
      <Card className="min-h-0 border-slate-800 bg-slate-950/80">
        <CardHeader>
          <CardTitle>Transaction history</CardTitle>
          <span className="text-[11px] text-slate-500">
            {history.length} entries · simulators + live network feed
          </span>
        </CardHeader>
        <CardContent className="overflow-auto">
          <table className="w-full text-left text-xs">
            <thead className="text-slate-500">
              <tr>
                <th className="pb-2 font-medium">Date</th>
                <th className="pb-2 font-medium">Rail</th>
                <th className="pb-2 font-medium">Corridor</th>
                <th className="pb-2 font-medium">Amount</th>
                <th className="pb-2 font-medium">Currency</th>
                <th className="pb-2 font-medium">Status</th>
                <th className="pb-2 font-medium">Proof</th>
                <th className="pb-2 font-medium" />
              </tr>
            </thead>
            <tbody>
              {history.length === 0 && (
                <tr>
                  <td colSpan={8} className="py-8 text-center text-slate-500">
                    No activity yet. Run a simulation or wait for the live network feed.
                  </td>
                </tr>
              )}
              {history.map((row) => (
                <tr key={row.id} className="border-t border-slate-800/80">
                  <td className="py-2.5 text-slate-300">{row.date}</td>
                  <td className="text-slate-400">
                    {row.route === "LegacySwift"
                      ? "SWIFT"
                      : row.route === "SovereignRs"
                        ? "Sovereign"
                        : row.route === "Network"
                          ? "Network"
                          : "—"}
                  </td>
                  <td className="max-w-[140px] truncate text-slate-400" title={row.corridor}>
                    {row.corridor ?? "—"}
                  </td>
                  <td className="font-mono">{row.amount}</td>
                  <td>{row.currency}</td>
                  <td>
                    <Badge tone={row.status === "Settled" ? "emerald" : "rose"}>{row.status}</Badge>
                  </td>
                  <td>
                    {row.verified && row.status === "Settled" ? (
                      <span className="inline-flex items-center gap-1 text-emerald-400">
                        <BadgeCheck className="h-3.5 w-3.5" /> Verified
                      </span>
                    ) : (
                      <span className="text-slate-500">—</span>
                    )}
                  </td>
                  <td>
                    <button
                      type="button"
                      className="inline-flex items-center gap-1 text-blue-300 hover:underline"
                      onClick={() => setOpenHash(row.tx_hash)}
                    >
                      Explorer <ExternalLink className="h-3 w-3" />
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {openHash && (
            <div className="mt-4 rounded-lg border border-slate-700 bg-slate-950 p-3">
              <div className="text-[11px] uppercase tracking-wider text-slate-500">Binding hash</div>
              <div className="mt-1 break-all font-mono text-[11px] text-slate-200">{openHash}</div>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
