# ID: packages/math/src/segment.ts:69
export const findSegmentIntersection = <Point extends number[]>(
  first: readonly [Point, Point],
  second: readonly [Point, Point],
): Point | null => {
  const p0 = vectorFromPoint(first[0]);
  const p1 = vectorFromPoint(first[1]);
  const q0 = vectorFromPoint(second[0]);
  const q1 = vectorFromPoint(second[1]);

  const r = vectorSubtract(p1, p0);
  const s = vectorSubtract(q1, q0);
  const denominator = vectorCross(r, s);

  if (denominator === 0) {
    return null;
  }

  const between = vectorSubtract(
    vectorFromPoint(second[0]),
    vectorFromPoint(first[0]),
  );
  const u = vectorCross(between, r) / denominator;
  const t = vectorCross(between, s) / denominator;

  if (u === 0) {
    return null;
  }

  const hit = vectorAdd(p0, vectorScale(r, t));

  if (t >= 0 && t < 1 && u >= 0 && u < 1) {
    return pointFromVector<Point>(hit);
  }

  return null;
};
