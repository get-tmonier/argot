# ID: packages/common/src/points.ts:21
export const resizePointsAlongAxis = <Point extends number[]>(
  dimension: 0 | 1,
  newSize: number,
  points: readonly Point[],
  normalize: boolean,
): Point[] => {
  const values = points.map((point) => point[dimension]);
  const upper = Math.max(...values);
  const lower = Math.min(...values);
  const extent = upper - lower;
  const scale = extent === 0 ? 1 : newSize / extent;

  let smallestAfter = Infinity;

  const scaled = points.map((point): Point => {
    const scaledValue = point[dimension] * scale;
    const copy = [...point];
    copy[dimension] = scaledValue;
    if (scaledValue < smallestAfter) {
      smallestAfter = scaledValue;
    }
    return copy as Point;
  });

  if (!normalize) {
    return scaled;
  }

  if (scaled.length === 2) {
    // two-point lines are left untranslated
    return scaled;
  }

  const shift = lower - smallestAfter;

  return scaled.map((point) =>
    pointFromPair<Point>(
      point.map((value, axis) =>
        axis === dimension ? value + shift : value,
      ) as [number, number],
    ),
  );
};
