import type { ReactNode } from "react";
import { Activity, ArrowLeftRight, ShieldCheck, Wallet } from "lucide-react";
import { SovereignLogo } from "@/components/SovereignLogo";
import { useDesk } from "@/store/useDesk";
import type { DeskView } from "@/lib/types";
import { cn } from "@/lib/utils";

const NAV: Array<{ id: DeskView; label: string; icon: typeof Activity }> = [
  { id: "simulator", label: "Simulator", icon: ArrowLeftRight },
  { id: "network", label: "Network", icon: Activity },
  { id: "wallet", label: "Wallet", icon: Wallet },
  { id: "compliance", label: "Compliance", icon: ShieldCheck },
];

export function Shell({ children }: { children: ReactNode }) {
  const view = useDesk((state) => state.view);
  const setView = useDesk((state) => state.setView);
  const stats = useDesk((state) => state.stats);

  return (
    <div className="flex h-full flex-col">
      <header className="flex h-14 items-center justify-between border-b border-desk-line px-5">
        <div className="flex items-center gap-3">
          <SovereignLogo size={36} className="rounded-md ring-1 ring-slate-800" />
          <div>
            <div className="text-sm font-semibold tracking-wide text-slate-100">SovereignPay</div>
            <div className="text-[11px] text-slate-400">Money freedom · compliant CBDC rails</div>
          </div>
        </div>
        <nav className="flex items-center gap-1 rounded-lg border border-desk-line bg-slate-950/50 p-1">
          {NAV.map((item) => {
            const Icon = item.icon;
            const active = view === item.id;
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => setView(item.id)}
                className={cn(
                  "flex items-center gap-2 rounded-md px-3 py-1.5 text-xs",
                  active ? "bg-slate-800 text-white" : "text-slate-400 hover:text-slate-200",
                )}
              >
                <Icon className="h-3.5 w-3.5" />
                {item.label}
              </button>
            );
          })}
        </nav>
        <div className="text-right text-[11px] text-slate-400">
          <div className="font-mono text-emerald-300">{stats ? `${stats.tps.toLocaleString()} TPS` : "—"}</div>
          <div>demonstration feed</div>
        </div>
      </header>
      <main className="min-h-0 flex-1 p-4">{children}</main>
    </div>
  );
}
