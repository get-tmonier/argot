# ID: packages/math/src/angle.ts:47
export function angleWithinRange(
  angle: Radians,
  lower: Radians,
  upper: Radians,
): boolean {
  angle = normalizeRadians(angle);
  lower = normalizeRadians(lower);
  upper = normalizeRadians(upper);

  if (lower < upper) {
    return angle >= lower && angle <= upper;
  }

  // the arc wraps past the zero angle
  return angle >= lower || angle <= upper;
}
