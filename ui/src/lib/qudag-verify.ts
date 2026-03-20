import type { VerificationResult } from "./types";

// ---------------------------------------------------------------------------
// WASM module interface — matches wasm-bindgen output
// ---------------------------------------------------------------------------

interface QuDagWasm {
  verify_dilithium_signature(data: Uint8Array, signature: Uint8Array, public_key: Uint8Array): boolean;
  verify_dag_parents(message_json: string, known_tips_json: string): boolean;
  verify_proposal_votes(proposal_json: string): VerificationResult;
}

let wasmModule: QuDagWasm | null = null;
let wasmLoadPromise: Promise<QuDagWasm> | null = null;

/**
 * Lazily load and initialise the QuDAG WASM module.
 *
 * The WASM binary is expected at `/pkg/verdant_qudag_wasm.js` (the
 * wasm-pack `--target web` output). Vite serves this from `public/pkg/`.
 */
async function loadWasm(): Promise<QuDagWasm> {
  if (wasmModule) return wasmModule;
  if (wasmLoadPromise) return wasmLoadPromise;

  wasmLoadPromise = (async () => {
    // Dynamic import — Vite serves from public/pkg/ at runtime
    // @ts-expect-error — runtime-only module produced by wasm-pack
    const mod = await import(/* @vite-ignore */ "/pkg/verdant_qudag_wasm.js");
    await mod.default();           // initialise WASM
    wasmModule = mod as QuDagWasm;
    return wasmModule;
  })();

  return wasmLoadPromise;
}

// ---------------------------------------------------------------------------
// Public TypeScript API
// ---------------------------------------------------------------------------

/**
 * Verify a CRYSTALS-Dilithium post-quantum signature.
 */
export async function verifyDilithiumSignature(
  data: Uint8Array,
  signature: Uint8Array,
  publicKey: Uint8Array,
): Promise<boolean> {
  const wasm = await loadWasm();
  return wasm.verify_dilithium_signature(data, signature, publicKey);
}

/**
 * Verify that a message's DAG parents are all present in the known tip set.
 */
export async function verifyDagParents(
  messageJson: string,
  knownTipsJson: string,
): Promise<boolean> {
  const wasm = await loadWasm();
  return wasm.verify_dag_parents(messageJson, knownTipsJson);
}

/**
 * Verify the cryptographic integrity of all votes on a governance proposal.
 *
 * @param proposalJson - JSON string with `proposal_id` and `votes` array
 * @returns Verification result with valid flag, total votes, and discrepancy count
 */
export async function verifyProposalVotes(
  proposalJson: string,
): Promise<VerificationResult> {
  const wasm = await loadWasm();
  return wasm.verify_proposal_votes(proposalJson);
}
