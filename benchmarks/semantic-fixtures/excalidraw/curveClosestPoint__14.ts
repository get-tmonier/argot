# ID: packages/math/src/curve.ts:227
export function nearestPointOnCurve<Point extends number[]>(
  c: Curve<Point>,
  p: Point,
  tolerance: number = 1e-3,
): Point | null {
  const ternarySearch = (
    lo: number,
    hi: number,
    f: (t: number) => number,
    eps: number = tolerance,
  ) => {
    let left = lo;
    let right = hi;
    let mid;

    while (right - left > eps) {
      mid = (right + left) / 2;
      if (f(mid - eps) < f(mid + eps)) {
        right = mid;
      } else {
        left = mid;
      }
    }

    return mid;
  };

  const steps = 30;
  let bestStep = 0;
  for (let best = Infinity, i = 0; i < steps; i++) {
    const d = pointDistance(p, bezierEquation(c, i / steps));
    if (d < best) {
      best = d;
      bestStep = i;
    }
  }

  const lower = Math.max((bestStep - 1) / steps, 0);
  const upper = Math.min((bestStep + 1) / steps, 1);
  const t = ternarySearch(lower, upper, (u) =>
    pointDistance(p, bezierEquation(c, u)),
  );

  if (!t) {
    return null;
  }

  return bezierEquation(c, t);
}
