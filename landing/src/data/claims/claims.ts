import manifest from './manifest.json';
import { type ClaimManifest, type ClaimRecord, validateManifest } from './schema';

const claims = validateManifest(manifest as ClaimManifest);

export function claim(key: string): ClaimRecord {
  const record = claims.claims.find((candidate) => candidate.key === key);
  if (!record) throw new Error(`unknown claim key: ${key}`);
  return record;
}

export function claimValue(key: string): string {
  const record = claim(key);
  if (record.status === 'unavailable') return record.qualifier;
  if (
    record.numerator === undefined ||
    record.denominator === undefined ||
    record.percentage === undefined
  )
    throw new Error(`numeric claim is incomplete: ${key}`);
  return `${record.numerator}/${record.denominator} (${record.percentage.toFixed(1)}%) — ${record.qualifier}`;
}
