# ID: src/internal/group-by.ts:36
function groupValuesBy<TOriginalValue, TMappedValue>(
  values: ReadonlyArray<TOriginalValue>,
  keyMapper: (value: TOriginalValue) => string | number,
  valueMapper: (value: TOriginalValue) => TMappedValue = (value) =>
    value as unknown as TMappedValue
): Record<string, TMappedValue[]> {
  const buckets: Record<string, TMappedValue[]> = {};

  for (const value of values) {
    const key = keyMapper(value);
    if (buckets[key] === undefined) {
      buckets[key] = [];
    }

    buckets[key].push(valueMapper(value));
  }

  return buckets;
}
