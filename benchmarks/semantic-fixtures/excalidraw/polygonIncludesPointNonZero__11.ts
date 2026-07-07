# ID: packages/math/src/polygon.ts:44
export const pointInPolygonWinding = <Point extends [number, number]>(
  point: Point,
  polygon: Point[],
): boolean => {
  const [x, y] = point;
  let winding = 0;

  for (let i = 0; i < polygon.length; i++) {
    const next = (i + 1) % polygon.length;
    const [xi, yi] = polygon[i];
    const [xj, yj] = polygon[next];

    if (yi <= y) {
      if (yj > y && (xj - xi) * (y - yi) - (x - xi) * (yj - yi) > 0) {
        winding++;
      }
    } else if (yj <= y && (xj - xi) * (y - yi) - (x - xi) * (yj - yi) < 0) {
      winding--;
    }
  }

  return winding !== 0;
};
