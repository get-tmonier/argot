# ID: packages/element/src/sizeHelpers.ts:225
export const normalizeElementBox = (element: {
  width: number;
  height: number;
  x: number;
  y: number;
}): { width: number; height: number; x: number; y: number } => {
  const result = {
    width: element.width,
    height: element.height,
    x: element.x,
    y: element.y,
  };

  if (element.width < 0) {
    const positiveWidth = Math.abs(element.width);
    result.width = positiveWidth;
    result.x = element.x - positiveWidth;
  }

  if (element.height < 0) {
    const positiveHeight = Math.abs(element.height);
    result.height = positiveHeight;
    result.y = element.y - positiveHeight;
  }

  return result;
};
