import type { CountryNode, GlobalNetworkSnapshot, Region, RegionActivity } from "./types";

export const COUNTRY_CATALOG: CountryNode[] = [
  country("br", "Brazil", "SouthAmerica", "Banco Central do Brasil", "BRL", "Drex", false, true, "🇧🇷", 32, 72),
  country("us", "United States", "NorthAmerica", "Federal Reserve", "USD", "Digital USD (pilot)", false, true, "🇺🇸", 22, 42),
  country("de", "Germany", "Europe", "Deutsche Bundesbank / ECB", "EUR", "Digital Euro", false, true, "🇩🇪", 50, 34),
  country("cn", "China", "Asia", "People's Bank of China", "CNY", "e-CNY", false, true, "🇨🇳", 78, 40),
  country("ru", "Russia", "Europe", "Bank of Russia", "RUB", "Digital Ruble", true, false, "🇷🇺", 62, 28),
  country("ir", "Iran", "MiddleEast", "Central Bank of Iran", "IRR", "Digital Rial", true, false, "🇮🇷", 58, 46),
  country("ae", "United Arab Emirates", "MiddleEast", "Central Bank of the UAE", "AED", "Digital Dirham", false, true, "🇦🇪", 60, 50),
  country("za", "South Africa", "Africa", "South African Reserve Bank", "ZAR", "Project Khokha", false, true, "🇿🇦", 52, 78),
  country("in", "India", "Asia", "Reserve Bank of India", "INR", "Digital Rupee", false, true, "🇮🇳", 68, 52),
  country("ng", "Nigeria", "Africa", "Central Bank of Nigeria", "NGN", "eNaira", false, true, "🇳🇬", 48, 58),
];

function country(
  id: string,
  name: string,
  region: Region,
  centralBank: string,
  fiat: string,
  cbdc: string,
  sanctioned: boolean,
  swiftMember: boolean,
  flag: string,
  mapX: number,
  mapY: number,
): CountryNode {
  return {
    id,
    name,
    region,
    centralBank,
    fiatCurrency: fiat,
    cbdcName: cbdc,
    isSanctioned: sanctioned,
    swiftMember: swiftMember,
    flag,
    mapX,
    mapY,
  };
}

export function findCountry(id: string): CountryNode | undefined {
  return COUNTRY_CATALOG.find((item) => item.id === id);
}

export interface CorridorContext {
  origin: CountryNode;
  dest: CountryNode;
  sanctionedTouch: boolean;
  usdCorridor: boolean;
  highSwiftFriction: boolean;
}

export function corridorContext(originId: string, destId: string): CorridorContext | null {
  const origin = findCountry(originId);
  const dest = findCountry(destId);
  if (!origin || !dest) return null;
  const sanctionedTouch = origin.isSanctioned || dest.isSanctioned;
  const usdCorridor = origin.fiatCurrency === "USD" || dest.fiatCurrency === "USD";
  const highSwiftFriction = sanctionedTouch || usdCorridor || !origin.swiftMember || !dest.swiftMember;
  return { origin, dest, sanctionedTouch, usdCorridor, highSwiftFriction };
}

export function legacyFee(ctx: CorridorContext): number {
  let fee = 45;
  if (ctx.usdCorridor) fee += 25;
  if (ctx.sanctionedTouch) fee += 80;
  if (!ctx.origin.swiftMember || !ctx.dest.swiftMember) fee += 30;
  return Math.min(fee, 150);
}

export const US_LISTED_CORRIDOR_NOTICE =
  "This corridor touches a jurisdiction often blocked on US correspondent SWIFT rails. Legacy SWIFT may freeze or delay; Sovereign settles on open participant CBDC rails without a dollar nostro hop.";

export const SOVEREIGN_BLOCKED_FOOTER =
  "Compliance halt on this rail. Choose another corridor or retry after policy review.";

export function riskLabels(ctx: CorridorContext | null) {
  if (!ctx) {
    return {
      swift: { tone: "slate" as const, text: "Select origin and destination" },
      sovereign: { tone: "slate" as const, text: "Select origin and destination" },
    };
  }
  if (ctx.sanctionedTouch) {
    return {
      swift: { tone: "rose" as const, text: "🔴 High US-correspondent / OFAC friction" },
      sovereign: {
        tone: "emerald" as const,
        text: "🟢 Open sovereign route (no US SWIFT gate)",
      },
    };
  }
  if (ctx.highSwiftFriction) {
    return {
      swift: { tone: "amber" as const, text: "🟡 Elevated friction (USD / correspondent)" },
      sovereign: { tone: "emerald" as const, text: "🟢 Direct CBDC corridor available" },
    };
  }
  return {
    swift: { tone: "amber" as const, text: "🟡 SWIFT correspondent (2–5 days)" },
    sovereign: { tone: "emerald" as const, text: "🟢 Sovereign settlement (< 2s demo)" },
  };
}

export function mockGlobalSnapshot(): GlobalNetworkSnapshot {
  return {
    regions: [
      region("SouthAmerica", "América do Sul", 3),
      region("NorthAmerica", "América do Norte", 2),
      region("Europe", "Europa", 4),
      region("Asia", "Ásia", 5),
      region("MiddleEast", "Oriente Médio", 3),
      region("Africa", "África", 2),
    ],
    corridors: [
      { pair: "CNY ↔ BRL", volumeUsd: "$450M hoje" },
      { pair: "AED ↔ EUR", volumeUsd: "$210M hoje" },
      { pair: "INR ↔ ZAR", volumeUsd: "$95M hoje" },
      { pair: "USD ↔ BRL", volumeUsd: "$380M hoje" },
    ],
  };
}

function region(name: Region, label: string, activeNodes: number): RegionActivity {
  return { region: name, label, activeNodes };
}
