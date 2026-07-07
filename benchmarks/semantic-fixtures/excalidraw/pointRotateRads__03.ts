# ID: packages/math/src/point.ts:125
export function rotatePointRadians<Point extends number[]>(
  target: Point,
  pivot: Point,
  theta: number,
): Point {
  if (!theta) {
    return target;
  }

  const [px, py] = target;
  const [cx, cy] = pivot;

  return pointFrom(
    (px - cx) * Math.cos(theta) - (py - cy) * Math.sin(theta) + cx,
    (px - cx) * Math.sin(theta) + (py - cy) * Math.cos(theta) + cy,
  );
}
