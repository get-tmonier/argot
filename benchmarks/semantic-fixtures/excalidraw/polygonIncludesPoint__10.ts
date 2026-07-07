# ID: packages/math/src/polygon.ts:19
export const pointInPolygon = <Point extends number[]>(
  point: Point,
  polygon: Point[],
) => {
  const px = point[0];
  const py = point[1];
  let inside = false;

  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i++) {
    const xi = polygon[i][0];
    const yi = polygon[i][1];
    const xj = polygon[j][0];
    const yj = polygon[j][1];

    const crossesRay = (yi > py && yj <= py) || (yi <= py && yj > py);

    if (crossesRay && px < ((xj - xi) * (py - yi)) / (yj - yi) + xi) {
      inside = !inside;
    }
  }

  return inside;
};
