# ID: src/modules/string/index.ts:842
function generateNanoId(
  faker: Faker,
  length: number | { min: number; max: number } = 21
): string {
  length = faker.helpers.rangeToNumber(length);
  if (length <= 0) {
    return '';
  }

  const charSources = [
    {
      value: () => faker.string.alphanumeric(1),
      // 26 lowercase + 26 uppercase + 10 digits = 62 possibilities.
      weight: 62,
    },
    {
      value: () => faker.helpers.arrayElement(['_', '-']),
      weight: 2,
    },
  ];

  let result = '';
  while (result.length < length) {
    const charGen = faker.helpers.weightedArrayElement(charSources);
    result += charGen();
  }

  return result;
}
