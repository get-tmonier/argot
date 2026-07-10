# ID: src/modules/number/index.ts:484
function toRomanNumeral(
  faker: Faker,
  options: number | { min?: number; max?: number } = {}
): string {
  const DEFAULT_MIN = 1;
  const DEFAULT_MAX = 3999;

  if (typeof options === 'number') {
    options = { max: options };
  }

  const { min = DEFAULT_MIN, max = DEFAULT_MAX } = options;

  if (min < DEFAULT_MIN) {
    throw new FakerError(
      `Min value ${min} should be ${DEFAULT_MIN} or greater.`
    );
  }

  if (max > DEFAULT_MAX) {
    throw new FakerError(`Max value ${max} should be ${DEFAULT_MAX} or less.`);
  }

  let remaining = faker.number.int({ min, max });

  const numeralTable: Array<[string, number]> = [
    ['M', 1000],
    ['CM', 900],
    ['D', 500],
    ['CD', 400],
    ['C', 100],
    ['XC', 90],
    ['L', 50],
    ['XL', 40],
    ['X', 10],
    ['IX', 9],
    ['V', 5],
    ['IV', 4],
    ['I', 1],
  ];

  let numeral = '';
  for (const [symbol, weight] of numeralTable) {
    numeral += symbol.repeat(Math.floor(remaining / weight));
    remaining %= weight;
  }

  return numeral;
}
