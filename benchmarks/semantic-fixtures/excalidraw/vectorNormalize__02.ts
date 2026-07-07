# ID: packages/math/src/vector.ts:147
export const unitVector = (input: Vector): Vector => {
  const length = vectorMagnitude(input);

  if (length === 0) {
    return vector(0, 0);
  }

  return vector(input[0] / length, input[1] / length);
};
