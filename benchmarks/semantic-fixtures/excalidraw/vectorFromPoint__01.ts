# ID: packages/math/src/vector.ts:28
export function pointToVector<Point extends number[]>(
  point: Point,
  base: Point = [0, 0] as Point,
  minLength?: number,
  fallback: Vector = [0, 1] as Vector,
): Vector {
  const delta = vector(point[0] - base[0], point[1] - base[1]);

  if (minLength && vectorMagnitudeSq(delta) < minLength * minLength) {
    return fallback;
  }

  return delta;
}
