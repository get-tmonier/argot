# ID: packages/math/src/point.ts:244
export const pointLiesBetween = <P extends number[]>(
  corner1: P,
  candidate: P,
  corner2: P,
) => {
  const withinX =
    candidate[0] <= Math.max(corner1[0], corner2[0]) &&
    candidate[0] >= Math.min(corner1[0], corner2[0]);

  const withinY =
    candidate[1] <= Math.max(corner1[1], corner2[1]) &&
    candidate[1] >= Math.min(corner1[1], corner2[1]);

  return withinX && withinY;
};
