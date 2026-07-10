# ID: packages/math/src/segment.ts:116
export const pointSegmentDistance = <Point extends number[]>(
  point: Point,
  segment: [Point, Point],
) => {
  const [px, py] = point;
  const [[ax, ay], [bx, by]] = segment;

  const dpx = px - ax;
  const dpy = py - ay;
  const segX = bx - ax;
  const segY = by - ay;

  const projection = dpx * segX + dpy * segY;
  const segLenSq = segX * segX + segY * segY;
  let ratio = -1;
  if (segLenSq !== 0) {
    ratio = projection / segLenSq;
  }

  let nearestX;
  let nearestY;

  if (ratio < 0) {
    nearestX = ax;
    nearestY = ay;
  } else if (ratio > 1) {
    nearestX = bx;
    nearestY = by;
  } else {
    nearestX = ax + ratio * segX;
    nearestY = ay + ratio * segY;
  }

  const deltaX = px - nearestX;
  const deltaY = py - nearestY;
  return Math.sqrt(deltaX * deltaX + deltaY * deltaY);
};
