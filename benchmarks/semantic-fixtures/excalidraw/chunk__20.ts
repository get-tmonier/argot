# ID: packages/common/src/utils.ts:208
export const splitIntoChunks = <T>(array: readonly T[], size: number): T[][] => {
  if (!array.length || size < 1) {
    return [];
  }

  let cursor = 0;
  let outIndex = 0;
  const result = Array(Math.ceil(array.length / size));

  while (cursor < array.length) {
    result[outIndex++] = array.slice(cursor, (cursor += size));
  }

  return result;
};
