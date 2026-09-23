import type { HTMLAttributes } from "react";
import { cn } from "@/lib/utils";

const tones = {
  emerald: "bg-emerald-500/15 text-emerald-300 ring-emerald-500/30",
  amber: "bg-amber-500/15 text-amber-300 ring-amber-500/30",
  rose: "bg-rose-500/15 text-rose-300 ring-rose-500/30",
  blue: "bg-blue-500/15 text-blue-300 ring-blue-500/30",
  slate: "bg-slate-500/15 text-slate-300 ring-slate-500/30",
};

export function Badge({
  tone = "slate",
  className,
  ...props
}: HTMLAttributes<HTMLSpanElement> & { tone?: keyof typeof tones }) {
  return (
    <span
      className={cn("inline-flex items-center rounded-full px-2 py-0.5 text-[11px] font-medium ring-1", tones[tone], className)}
      {...props}
    />
  );
}
