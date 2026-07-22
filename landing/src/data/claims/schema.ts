export type ClaimStatus = "candidate" | "canonical" | "unavailable";

export interface ClaimRecord {
  key: string;
  status: ClaimStatus;
  source: string;
  revision: string;
  observedAt: string;
  scope: string;
  qualifier: string;
  numerator?: number;
  denominator?: number;
  percentage?: number;
  supersedes?: string[];
  unavailableReason?: string;
}

export interface ClaimManifest {
  schemaVersion: 1;
  generatedAt: string;
  claims: ClaimRecord[];
}

function fail(message: string): never {
  throw new Error(`invalid claim manifest: ${message}`);
}

export function percentage(numerator: number, denominator: number): number {
  if (!Number.isInteger(numerator) || !Number.isInteger(denominator) || numerator < 0 || denominator <= 0 || numerator > denominator) {
    fail("numerator and denominator must be non-negative integers with numerator <= denominator");
  }
  return (numerator / denominator) * 100;
}

export function validateManifest(manifest: ClaimManifest): ClaimManifest {
  if (manifest.schemaVersion !== 1 || !manifest.generatedAt || !Array.isArray(manifest.claims)) fail("schemaVersion, generatedAt, and claims are required");
  const keys = new Set<string>();
  for (const claim of manifest.claims) {
    for (const field of ["key", "source", "revision", "observedAt", "scope", "qualifier"] as const) if (!claim[field]) fail(`${field} is required for ${claim.key || "a claim"}`);
    if (keys.has(claim.key)) fail(`duplicate key ${claim.key}`);
    keys.add(claim.key);
    const hasCounts = claim.numerator !== undefined || claim.denominator !== undefined || claim.percentage !== undefined;
    if (claim.status === "unavailable") {
      if (!claim.unavailableReason || hasCounts) fail(`unavailable claim ${claim.key} requires a reason and no numeric value`);
      continue;
    }
    if (!hasCounts || claim.numerator === undefined || claim.denominator === undefined || claim.percentage === undefined) fail(`numeric claim ${claim.key} requires numerator, denominator, and percentage`);
    if (Math.abs(percentage(claim.numerator, claim.denominator) - claim.percentage) > 1e-9) fail(`percentage does not match ${claim.key}`);
  }
  return manifest;
}
