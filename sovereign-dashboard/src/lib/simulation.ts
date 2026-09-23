import type { RouteType, SimulatePaymentRequest, TransactionUpdate } from "./types";

type Emit = (update: TransactionUpdate) => void;

function restricted(name: string): boolean {
  const value = name.toLowerCase();
  return value.includes("restricted") || value.includes("sanction");
}

function demoHash(seed: string): string {
  let acc = 0xcbf29ce484222325n;
  for (const byte of new TextEncoder().encode(seed)) {
    acc ^= BigInt(byte);
    acc = (acc * 0x100000001b3n) & ((1n << 64n) - 1n);
  }
  return `0x${acc.toString(16).padStart(16, "0").repeat(2)}`;
}

function wait(ms: number) {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

const LEGACY = [
  { label: "Routing via Correspondent Bank (NY)", detail: "Instruction queued at the nostro agent in New York", tone: "amber" as const, delay: 1400, block: false },
  { label: "Waiting for USD Clearing", detail: "Dollar leg parked in the correspondent chain", tone: "amber" as const, delay: 1500, block: false },
  { label: "OFAC / Sanctions Compliance Check", detail: "Name screen pending at the intermediary", tone: "amber" as const, delay: 1600, block: true },
  { label: "Intermediary Fees Applied", detail: "Correspondent and FX mark-ups deducted", tone: "amber" as const, delay: 1300, block: false },
  { label: "Final Settlement", detail: "Beneficiary credit after the chain completes", tone: "amber" as const, delay: 1200, block: false },
];

const SOVEREIGN = [
  { label: "ZK-Proof Compliance Validation", detail: "Public limit and sanctions-clear flag verified; amount stays private", tone: "blue" as const, delay: 350, block: true },
  { label: "Lock BRL-CBDC in Smart Contract", detail: "Source units locked by the issuing central bank", tone: "blue" as const, delay: 400, block: false },
  { label: "Execute Atomic Swap (Oracle Price)", detail: "Both legs settle together or the batch aborts", tone: "emerald" as const, delay: 450, block: false },
  { label: "BFT Consensus Reached (67% Nodes)", detail: "Stake-weighted certificate from the local validator set", tone: "emerald" as const, delay: 400, block: false },
  { label: "Final Irreversible Settlement", detail: "Proof binding published to the explorer commitment", tone: "emerald" as const, delay: 350, block: false },
];

export async function simulatePaymentBrowser(
  request: SimulatePaymentRequest,
  emit: Emit,
): Promise<string> {
  const paymentId = request.paymentId ?? crypto.randomUUID();
  const blocked = restricted(request.from) || restricted(request.to);
  const pipeline = request.route === "LegacySwift" ? LEGACY : SOVEREIGN;
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
    if (blocked && def.block) {
      emit({
        paymentId,
        route: request.route,
        kind: "blocked",
        step: {
          index,
          total: pipeline.length,
          label: def.label,
          detail: "Counterparty failed the sanctions screen. No proof is built.",
          tone: "rose",
          status: "blocked",
        },
        elapsedMs: Math.round(performance.now() - started),
        feeUsd: request.route === "SovereignRs" ? 0 : 45,
        txHash: null,
        totalElapsedMs: Math.round(performance.now() - started),
        timestamp: stamp(),
      });
      throw new Error("counterparty failed the sanctions screen");
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
  const fee = request.route === "LegacySwift" ? 45 : 0.01;
  const txHash = request.route === "SovereignRs" ? demoHash(paymentId) : null;
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

export function emptyTrack(): import("./types").TrackSnapshot {
  return { phase: "idle", steps: [], feeUsd: null, txHash: null, totalMs: null };
}

export function applyUpdate(
  track: import("./types").TrackSnapshot,
  update: TransactionUpdate,
): import("./types").TrackSnapshot {
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
