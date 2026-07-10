# ID: packages/math/src/segment.ts:181
export function segmentToSegmentDistance<Point extends number[]>(
  first: [Point, Point],
  second: [Point, Point],
): number {
  if (lineSegmentIntersectionPoints(first, second)) {
    return 0;
  }

  return Math.min(
    distanceToLineSegment(first[0], second),
    distanceToLineSegment(first[1], second),
    distanceToLineSegment(second[0], first),
    distanceToLineSegment(second[1], first),
  );
}
