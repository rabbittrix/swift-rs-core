import { useMemo, useState } from "react";
import type { CountryNode, Region } from "@/lib/types";
import { cn } from "@/lib/utils";

const REGION_ORDER: Region[] = [
  "SouthAmerica",
  "NorthAmerica",
  "Europe",
  "Asia",
  "MiddleEast",
  "Africa",
];

const REGION_LABEL: Record<Region, string> = {
  SouthAmerica: "South America",
  NorthAmerica: "North America",
  Europe: "Europe",
  Asia: "Asia",
  MiddleEast: "Middle East",
  Africa: "Africa",
};

export function CountrySelector({
  label,
  countries,
  valueId,
  onChange,
}: {
  label: string;
  countries: CountryNode[];
  valueId: string;
  onChange: (node: CountryNode) => void;
}) {
  const [query, setQuery] = useState("");
  const selected = countries.find((item) => item.id === valueId);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return countries;
    return countries.filter(
      (item) =>
        item.name.toLowerCase().includes(q) ||
        item.centralBank.toLowerCase().includes(q) ||
        item.fiatCurrency.toLowerCase().includes(q),
    );
  }, [countries, query]);

  const grouped = useMemo(() => {
    const map = new Map<Region, CountryNode[]>();
    for (const region of REGION_ORDER) map.set(region, []);
    for (const item of filtered) {
      map.get(item.region)?.push(item);
    }
    return REGION_ORDER.map((region) => ({ region, items: map.get(region) ?? [] })).filter(
      (entry) => entry.items.length > 0,
    );
  }, [filtered]);

  return (
    <div className="space-y-2">
      <div className="text-[11px] font-medium uppercase tracking-wider text-slate-500">{label}</div>
      <input
        className="w-full rounded-md border border-slate-800 bg-slate-950 px-2 py-2 text-xs text-slate-200 placeholder:text-slate-600"
        placeholder="Search country, bank, or currency…"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      <div className="max-h-40 overflow-auto rounded-md border border-slate-800 bg-slate-950/80">
        {grouped.map(({ region, items }) => (
          <div key={region}>
            <div className="sticky top-0 bg-slate-900 px-2 py-1 text-[10px] font-semibold uppercase tracking-wider text-slate-500">
              {REGION_LABEL[region]}
            </div>
            {items.map((item) => {
              const active = item.id === valueId;
              return (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => {
                    onChange(item);
                    setQuery("");
                  }}
                  className={cn(
                    "flex w-full items-start gap-2 border-b border-slate-900/80 px-2 py-2 text-left text-xs transition-colors",
                    active ? "bg-blue-950/60 text-white" : "text-slate-300 hover:bg-slate-900",
                  )}
                >
                  <span className="text-base leading-none">{item.flag}</span>
                  <span className="min-w-0 flex-1">
                    <span className="block font-medium">{item.name}</span>
                    <span className="block truncate text-[11px] text-slate-500">{item.centralBank}</span>
                    <span className="font-mono text-[10px] text-slate-400">
                      {item.fiatCurrency} · {item.cbdcName}
                    </span>
                  </span>
                </button>
              );
            })}
          </div>
        ))}
      </div>
      {selected && (
        <div className="rounded-md border border-slate-800 bg-slate-950/60 px-2 py-2 text-xs text-slate-300">
          <span className="mr-1">{selected.flag}</span>
          {selected.centralBank} ({selected.fiatCurrency})
        </div>
      )}
    </div>
  );
}
