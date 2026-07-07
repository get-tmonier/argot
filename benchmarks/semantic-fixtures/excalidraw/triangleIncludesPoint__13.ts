# ID: packages/math/src/triangle.ts:14
export function pointInsideTriangle<P extends number[]>(
  [a, b, c]: [P, P, P],
  p: P,
): boolean {
  const edgeSign = (u: P, v: P, w: P) =>
    (u[0] - w[0]) * (v[1] - w[1]) - (v[0] - w[0]) * (u[1] - w[1]);

  const s1 = edgeSign(p, a, b);
  const s2 = edgeSign(p, b, c);
  const s3 = edgeSign(p, c, a);

  const anyNegative = s1 < 0 || s2 < 0 || s3 < 0;
  const anyPositive = s1 > 0 || s2 > 0 || s3 > 0;

  return !(anyNegative && anyPositive);
}
