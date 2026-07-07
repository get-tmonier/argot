# ID: packages/element/src/distance.ts:63
const rectanguloidPointDistance = (
  element: ExcalidrawRectanguloidElement,
  elementsMap: ElementsMap,
  p: GlobalPoint,
) => {
  const center = elementCenterPoint(element, elementsMap);

  // Rather than rotating the shape, rotate the query point by the negated
  // angle — the resulting distance is identical.
  const localPoint = pointRotateRads(p, center, -element.angle as Radians);

  const [sides, corners] = deconstructRectanguloidElement(element);

  return Math.min(
    ...sides.map((side) => distanceToLineSegment(localPoint, side)),
    ...corners
      .map((corner) => curvePointDistance(corner, localPoint))
      .filter((d): d is number => d !== null),
  );
};
