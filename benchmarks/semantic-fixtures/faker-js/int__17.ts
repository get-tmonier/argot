# ID: src/modules/number/index.ts:44
function randomInt(
  faker: Faker,
  options: number | { min?: number; max?: number; multipleOf?: number } = {}
): number {
  if (typeof options === 'number') {
    options = { max: options };
  }

  const { min = 0, max = Number.MAX_SAFE_INTEGER, multipleOf = 1 } = options;

  if (!Number.isInteger(multipleOf)) {
    throw new FakerError(`multipleOf should be an integer.`);
  }

  if (multipleOf <= 0) {
    throw new FakerError(`multipleOf should be greater than 0.`);
  }

  const lowMultiple = Math.ceil(min / multipleOf);
  const highMultiple = Math.floor(max / multipleOf);

  if (lowMultiple === highMultiple) {
    return lowMultiple * multipleOf;
  }

  if (highMultiple < lowMultiple) {
    if (max >= min) {
      throw new FakerError(
        `No suitable integer value between ${min} and ${max} found.`
      );
    }

    throw new FakerError(`Max ${max} should be greater than min ${min}.`);
  }

  const { randomizer } = faker.fakerCore;
  const unit = randomizer.next();
  const span = highMultiple - lowMultiple + 1; // +1 keeps the max bound inclusive
  return Math.floor(unit * span + lowMultiple) * multipleOf;
}
