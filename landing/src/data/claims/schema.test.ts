import { expect, test } from "bun:test";
import { validateManifest } from "./schema";

const claim = { key: "foreign.example", status: "candidate" as const, source: "landing/src/data/foreign.json", revision: "7be8b1eb", observedAt: "2026-07-14", scope: "curated fixtures", qualifier: "candidate only", numerator: 595, denominator: 605, percentage: (595 / 605) * 100 };

test("accepts provenance-bearing candidate claims", () => expect(validateManifest({ schemaVersion: 1, generatedAt: "2026-07-22", claims: [claim] })).toBeDefined());
test("rejects missing provenance and stale percentages", () => {
  expect(() => validateManifest({ schemaVersion: 1, generatedAt: "2026-07-22", claims: [{ ...claim, source: "" }] })).toThrow("source is required");
  expect(() => validateManifest({ schemaVersion: 1, generatedAt: "2026-07-22", claims: [{ ...claim, percentage: 98 }] })).toThrow("percentage does not match");
});
