# ID: packages/element/src/sizeHelpers.ts:127
export const computeConstrainedSize = (
  elementType: string,
  width: number,
  height: number,
): { width: number; height: number } => {
  const absWidth = Math.abs(width);
  const absHeight = Math.abs(height);

  if (
    elementType === "line" ||
    elementType === "arrow" ||
    elementType === "freedraw"
  ) {
    const snappedAngle =
      Math.round(Math.atan(absHeight / absWidth) / SHIFT_LOCKING_ANGLE) *
      SHIFT_LOCKING_ANGLE;
    if (snappedAngle === 0) {
      height = 0;
    } else if (snappedAngle === Math.PI / 2) {
      width = 0;
    } else {
      height = absWidth * Math.tan(snappedAngle) * Math.sign(height) || height;
    }
  } else if (elementType !== "selection") {
    height = absWidth * Math.sign(height);
  }

  return { width, height };
};
