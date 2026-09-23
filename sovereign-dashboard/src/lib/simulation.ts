import { corridorContext, findCountry, legacyFee } from "./countries";
import type { SimulatePaymentRequest, TrackSnapshot, TransactionUpdate } from "./types";

type Emit = (update: TransactionUpdate) => void;

function wait(ms: number) {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

function demoHash(seed: string): string {
  let acc = 0xcbf29ce484222325n;
  for (const byte of new TextEncoder().encode(seed)) {
    acc ^= BigInt(byte);
    acc = (acc * 0x100000001b3n) & ((1n << 64n) - 1n);
  }
  return `0x${acc.toString(16).padStart(16, "0").repeat(2)}`;
}

interface StepDef {
  label: string;
  detail: string;
  tone: "amber" | "emerald" | "blue" | "rose";
  delay: number;
  blockHere: boolean;
}

function legacySteps(ctx: NonNullable<ReturnType<typeof corridorContext>>): StepDef[] {
  const steps: StepDef[] = [
    {
      label: `Routing via Correspondent Bank (${ctx.usdCorridor ? "NY" : "London"})`,
      detail: `${ctx.origin.fiatCurrency} → ${ctx.dest.fiatCurrency} instructed through a nostro agent`,
      tone: "amber",
      delay: ctx.highSwiftFriction ? 1800 : 1400,
      blockHere: false,
    },
  ];
  if (ctx.usdCorridor || ctx.highSwiftFriction) {
    steps.push({
      label: "Waiting for USD Clearing (New York)",
      detail: "Dollar leg parked while compliance queues the instruction",
      tone: "amber",
      delay: 2000,
      blockHere: false,
    });
  }
  if (ctx.sanctionedTouch) {
    steps.push(
      {
        label: "OFAC Sanctions Screening Triggered",
        detail: `Hit on ${ctx.origin.name} or ${ctx.dest.name} — enhanced due diligence required`,
        tone: "rose",
        delay: 2200,
        blockHere: false,
      },
      {
        label: "Correspondent Bank in NY Freezing Assets for Review",
        detail: "Funds held pending legal review; beneficiary credit blocked",
        tone: "rose",
        delay: 2400,
        blockHere: true,
      },
    );
  } else {
    steps.push({
      label: "OFAC / Sanctions Compliance Check",
      detail: "Name screen pending at the intermediary",
      tone: "amber",
      delay: 1600,
      blockHere: false,
    });
  }
  steps.push({
    label: "Intermediary Fees Applied",
    detail: "Correspondent, FX, and compliance mark-ups deducted",
    tone: "amber",
    delay: 1500,
    blockHere: false,
  });
  if (ctx.sanctionedTouch) {
    steps.push({
      label: "Transaction Rejected or Delayed (High Risk)",
      detail: "SWIFT chain will not release without manual exception (often denied)",
      tone: "rose",
      delay: 2000,
      blockHere: true,
    });
  } else {
    steps.push({
      label: "Final Settlement",
      detail: "Beneficiary credit after the correspondent chain completes (2–5 days)",
      tone: "amber",
      delay: 1800,
      blockHere: false,
    });
  }
  return steps;
}

function sovereignSteps(ctx: NonNullable<ReturnType<typeof corridorContext>>): StepDef[] {
  const head: StepDef[] = [
    {
      label: "ZK-Proof Compliance Validation",
      detail: ctx.sanctionedTouch
        ? "Listed jurisdiction detected — fail-closed screen; no proof is issued"
        : "Public limit and sanctions-clear flag verified; amount stays private",
      tone: ctx.sanctionedTouch ? "rose" : "blue",
      delay: 380,
      blockHere: ctx.sanctionedTouch,
    },
  ];
  if (ctx.sanctionedTouch) return head;
  return [
    ...head,
    {
      label: "Direct CBDC Corridor (No USD Correspondent)",
      detail: `Routing ${ctx.origin.cbdcName} → ${ctx.dest.cbdcName} without a dollar nostro hop`,
      tone: "blue",
      delay: 320,
      blockHere: false,
    },
    {
      label: "mBridge-Style Atomic Swap (Oracle Price)",
      detail: "Both CBDC legs lock and release in one batch",
      tone: "emerald",
      delay: 420,
      blockHere: false,
    },
    {
      label: "BFT Consensus Reached (67% Validators)",
      detail: "Regional central-bank validators co-sign the batch",
      tone: "emerald",
      delay: 380,
      blockHere: false,
    },
    {
      label: "Final Irreversible Settlement",
      detail: "Proof binding published; sub-2-second demo finality",
      tone: "emerald",
      delay: 300,
      blockHere: false,
    },
  ];
}

export async function simulatePaymentBrowser(
  request: SimulatePaymentRequest,
  emit: Emit,
): Promise<string> {
  const ctx = corridorContext(request.originCountryId, request.destinationCountryId);
  if (!ctx) throw new Error("unknown country");
  const paymentId = request.paymentId ?? crypto.randomUUID();
  const pipeline =
    request.route === "LegacySwift" ? legacySteps(ctx) : sovereignSteps(ctx);
  const started = performance.now();
  const stamp = () => new Date().toISOString();

  for (let index = 0; index < pipeline.length; index += 1) {
    const def = pipeline[index];
    emit({
      paymentId,
      route: request.route,
      kind: "step",
      step: {
        index,
        total: pipeline.length,
        label: def.label,
        detail: def.detail,
        tone: def.tone,
        status: "active",
      },
      elapsedMs: Math.round(performance.now() - started),
      feeUsd: null,
      txHash: null,
      totalElapsedMs: null,
      timestamp: stamp(),
    });
    await wait(def.delay);
    if (def.blockHere) {
      emit({
        paymentId,
        route: request.route,
        kind: "blocked",
        step: {
          index,
          total: pipeline.length,
          label: def.label,
          detail:
            request.route === "SovereignRs"
              ? "Sanctions list match — payment cannot be included in a block."
              : "Corridor frozen by compliance — settlement not released.",
          tone: "rose",
          status: "blocked",
        },
        elapsedMs: Math.round(performance.now() - started),
        feeUsd: request.route === "LegacySwift" ? legacyFee(ctx) : 0,
        txHash: null,
        totalElapsedMs: Math.round(performance.now() - started),
        timestamp: stamp(),
      });
      throw new Error("corridor blocked by compliance");
    }
    emit({
      paymentId,
      route: request.route,
      kind: "step",
      step: {
        index,
        total: pipeline.length,
        label: def.label,
        detail: def.detail,
        tone: def.tone,
        status: "done",
      },
      elapsedMs: Math.round(performance.now() - started),
      feeUsd: null,
      txHash: null,
      totalElapsedMs: null,
      timestamp: stamp(),
    });
  }
  const fee = request.route === "LegacySwift" ? legacyFee(ctx) : 0.04;
  const txHash =
    request.route === "SovereignRs" ? demoHash(`${paymentId}:${ctx.origin.id}:${ctx.dest.id}`) : null;
  emit({
    paymentId,
    route: request.route,
    kind: "complete",
    step: null,
    elapsedMs: Math.round(performance.now() - started),
    feeUsd: fee,
    txHash,
    totalElapsedMs: Math.round(performance.now() - started),
    timestamp: stamp(),
  });
  return paymentId;
}

export function emptyTrack(): TrackSnapshot {
  return { phase: "idle", steps: [], feeUsd: null, txHash: null, totalMs: null };
}

export function applyUpdate(track: TrackSnapshot, update: TransactionUpdate): TrackSnapshot {
  if (update.kind === "step" && update.step) {
    const steps = [...track.steps];
    steps[update.step.index] = update.step;
    const phase =
      update.step.status === "blocked"
        ? "blocked"
        : update.step.status === "active" || steps.some((s) => s?.status === "done")
          ? "running"
          : track.phase;
    return { ...track, phase, steps };
  }
  if (update.kind === "blocked") {
    const steps = update.step ? [...track.steps.slice(0, update.step.index), update.step] : track.steps;
    return {
      phase: "blocked",
      steps,
      feeUsd: update.feeUsd,
      txHash: null,
      totalMs: update.totalElapsedMs,
    };
  }
  if (update.kind === "complete") {
    return {
      phase: "settled",
      steps: track.steps,
      feeUsd: update.feeUsd,
      txHash: update.txHash,
      totalMs: update.totalElapsedMs,
    };
  }
  return track;
}

export function cbdcPair(originId: string, destId: string): { from: string; to: string } {
  const origin = findCountry(originId);
  const dest = findCountry(destId);
  return {
    from: origin?.cbdcName ?? "CBDC",
    to: dest?.cbdcName ?? "CBDC",
  };
}
