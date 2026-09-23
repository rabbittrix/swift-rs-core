import { cn } from "@/lib/utils";

/** Money-freedom mark: open wings + CBDC coin on sovereign rails. */
export function SovereignLogo({ className, size = 32 }: { className?: string; size?: number }) {
  return (
    <svg
      viewBox="0 0 64 64"
      width={size}
      height={size}
      className={cn("shrink-0", className)}
      aria-hidden
    >
      <defs>
        <linearGradient id="sp-rail" x1="8" y1="8" x2="56" y2="56" gradientUnits="userSpaceOnUse">
          <stop stopColor="#2563eb" />
          <stop offset="1" stopColor="#10b981" />
        </linearGradient>
        <linearGradient id="sp-coin" x1="20" y1="18" x2="44" y2="46" gradientUnits="userSpaceOnUse">
          <stop stopColor="#34d399" />
          <stop offset="1" stopColor="#60a5fa" />
        </linearGradient>
      </defs>
      <rect width="64" height="64" rx="14" className="fill-slate-950" />
      <path
        d="M12 36c8-14 32-14 40 0"
        stroke="url(#sp-rail)"
        strokeWidth="3"
        strokeLinecap="round"
        fill="none"
      />
      <path
        d="M14 34l-4 2 2-4M50 34l4 2-2-4"
        stroke="#34d399"
        strokeWidth="2.5"
        strokeLinecap="round"
        strokeLinejoin="round"
        fill="none"
      />
      <circle cx="32" cy="32" r="14" stroke="url(#sp-rail)" strokeWidth="2.5" fill="none" />
      <circle cx="32" cy="32" r="10" fill="url(#sp-coin)" opacity="0.45" />
      <path d="M32 26v12M28 30h8" stroke="#ecfdf5" strokeWidth="2.2" strokeLinecap="round" />
      <path d="M22 22l-3-3M42 22l3-3" stroke="#93c5fd" strokeWidth="2" strokeLinecap="round" />
    </svg>
  );
}
