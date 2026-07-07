# ID: packages/math/src/range.ts:57
export const intersectRanges = (
  [a0, a1]: InclusiveRange,
  [b0, b1]: InclusiveRange,
): InclusiveRange | null => {
  const lower = Math.max(a0, b0);
  const upper = Math.min(a1, b1);

  if (lower <= upper) {
    return toBrandedType<InclusiveRange>([lower, upper]);
  }

  return null;
};
