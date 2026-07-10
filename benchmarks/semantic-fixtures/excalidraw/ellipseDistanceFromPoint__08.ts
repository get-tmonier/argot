# ID: packages/math/src/ellipse.ts:88
export const distanceFromEllipse = <Point extends number[]>(
  p: Point,
  ellipse: { center: Point; halfWidth: number; halfHeight: number },
): number => {
  const { halfWidth, halfHeight, center } = ellipse;
  const a = halfWidth;
  const b = halfHeight;
  const local = vectorAdd(
    vectorFromPoint(p),
    vectorScale(vectorFromPoint(center), -1),
  );

  const absX = Math.abs(local[0]);
  const absY = Math.abs(local[1]);

  let tx = 0.707;
  let ty = 0.707;

  for (let step = 0; step < 3; step++) {
    const x = a * tx;
    const y = b * ty;

    const ex = ((a * a - b * b) * tx ** 3) / a;
    const ey = ((b * b - a * a) * ty ** 3) / b;

    const rx = x - ex;
    const ry = y - ey;
    const qx = absX - ex;
    const qy = absY - ey;

    const r = Math.hypot(ry, rx);
    const q = Math.hypot(qy, qx);

    tx = Math.min(1, Math.max(0, ((qx * r) / q + ex) / a));
    ty = Math.min(1, Math.max(0, ((qy * r) / q + ey) / b));
    const norm = Math.hypot(ty, tx);
    tx /= norm;
    ty /= norm;
  }

  const closestX = a * tx * Math.sign(local[0]);
  const closestY = b * ty * Math.sign(local[1]);

  return pointDistance(pointFromVector(local), pointFrom(closestX, closestY));
};
