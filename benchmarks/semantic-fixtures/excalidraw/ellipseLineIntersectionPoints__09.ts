# ID: packages/math/src/ellipse.ts:197
export function ellipseLineHits<Point extends number[]>(
  ellipse: { center: Point; halfWidth: number; halfHeight: number },
  segment: [Point, Point],
): Point[] {
  const { center, halfWidth, halfHeight } = ellipse;
  const [start, finish] = segment;
  const [cx, cy] = center;
  const sx = start[0] - cx;
  const sy = start[1] - cy;
  const fx = finish[0] - cx;
  const fy = finish[1] - cy;

  const a =
    Math.pow(fx - sx, 2) / Math.pow(halfWidth, 2) +
    Math.pow(fy - sy, 2) / Math.pow(halfHeight, 2);
  const b =
    2 *
    ((sx * (fx - sx)) / Math.pow(halfWidth, 2) +
      (sy * (fy - sy)) / Math.pow(halfHeight, 2));
  const c =
    Math.pow(sx, 2) / Math.pow(halfWidth, 2) +
    Math.pow(sy, 2) / Math.pow(halfHeight, 2) -
    1;

  const disc = Math.sqrt(Math.pow(b, 2) - 4 * a * c);
  const t1 = (-b + disc) / (2 * a);
  const t2 = (-b - disc) / (2 * a);

  const candidates = [
    pointFrom<Point>(sx + t1 * (fx - sx) + cx, sy + t1 * (fy - sy) + cy),
    pointFrom<Point>(sx + t2 * (fx - sx) + cx, sy + t2 * (fy - sy) + cy),
  ].filter((pt) => !isNaN(pt[0]) && !isNaN(pt[1]));

  if (candidates.length === 2 && pointsEqual(candidates[0], candidates[1])) {
    return [candidates[0]];
  }

  return candidates;
}
