import { expect, test } from 'bun:test';
import candidates from './candidates.json';
import { claim, claimValue } from './claims';
import integrityCandidates from './integrity-candidates.json';
import manifest from './manifest.json';
import { type ClaimManifest, validateManifest } from './schema';

const canonicalManifest = manifest as ClaimManifest;

test('canonical claims expose their value and qualifier through stable keys', () => {
  expect(claimValue('foreign.visible_symbol')).toBe(
    '622/637 (97.6%) — detector-specific fixture recall; not a product-wide accuracy claim',
  );
  expect(claim('performance.audit_timing').status).toBe('unavailable');
});

test('manifest keeps the selected lineages and rejects known stale denominators', () => {
  expect(validateManifest(canonicalManifest)).toBeDefined();
  expect(
    canonicalManifest.claims.find((record) => record.key === 'foreign.visible_symbol'),
  ).toMatchObject({
    numerator: 622,
    denominator: 637,
  });
  expect(
    canonicalManifest.claims.find((record) => record.key === 'architecture.layering'),
  ).toMatchObject({
    numerator: 264,
    denominator: 272,
  });
  // The semantic senses report separately: misplacement abstains where a repo
  // has no separable architecture, so its denominator covers only the corpora
  // it could evaluate. One aggregate over both would hide that.
  expect(
    canonicalManifest.claims.find((record) => record.key === 'semantic.reinvention'),
  ).toMatchObject({
    numerator: 477,
    denominator: 584,
  });
  expect(
    canonicalManifest.claims.find((record) => record.key === 'semantic.misplacement'),
  ).toMatchObject({
    numerator: 15285,
    denominator: 16032,
  });
  expect(canonicalManifest.claims.find((record) => record.key === 'integrity.catch')).toMatchObject(
    {
      numerator: 154,
      denominator: 164,
    },
  );
  expect(candidates.claims.some((record) => record.key === 'foreign.legacy')).toBe(true);
  expect(integrityCandidates.claims.some((record) => record.key === 'integrity.original')).toBe(
    true,
  );
  expect(() =>
    validateManifest({
      ...canonicalManifest,
      claims: canonicalManifest.claims.map((record) =>
        record.key === 'foreign.visible_symbol' ? { ...record, denominator: 700 } : record,
      ),
    }),
  ).toThrow('percentage does not match');
});
