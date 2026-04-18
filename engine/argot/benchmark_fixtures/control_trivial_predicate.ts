import { Effect } from 'effect';
import type { HunkRecord } from '#modules/extract-dataset/domain/hunk-record.ts';
import { RepoContext } from '#modules/repo-context/application/ports/out/repo-context.port.ts';

export const filterSupportedLanguages = (
  records: ReadonlyArray<HunkRecord>,
): Effect.Effect<ReadonlyArray<HunkRecord>, never, RepoContext> =>
  Effect.gen(function* () {
    const ctx = yield* RepoContext;
    const settings = yield* ctx.readSettings();
    return records.filter((r) => settings.enabledLanguages.includes(r.language));
  });

export const countHunksByLanguage = (
  records: ReadonlyArray<HunkRecord>,
): Record<string, number> => {
  const counts: Record<string, number> = {};
  for (const r of records) {
    counts[r.language] = (counts[r.language] ?? 0) + 1;
  }
  return counts;
};

export const isRecentRecord = (r: HunkRecord, nowMs: number): boolean =>
  nowMs - Number(r.authorDateIso) * 1000 < 30 * 24 * 60 * 60 * 1000;

export const selectRecentHunks = (
  records: ReadonlyArray<HunkRecord>,
): Effect.Effect<ReadonlyArray<HunkRecord>, never> =>
  Effect.gen(function* () {
    const now = yield* Effect.sync(() => Date.now());
    return records.filter((r) => isRecentRecord(r, now));
  });
