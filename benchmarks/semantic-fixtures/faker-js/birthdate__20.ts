# ID: src/modules/date/index.ts:529
function generateBirthdate(
  faker: Faker,
  options: {
    mode?: 'age' | 'year';
    min?: number;
    max?: number;
    refDate?: string | Date | number;
  } = {}
): Date {
  const {
    mode = 'age',
    min = 18,
    max = 80,
    refDate: rawRefDate = faker.defaultRefDate(),
  } = options;

  const refDate = toDate(rawRefDate);
  const refYear = refDate.getUTCFullYear();

  switch (mode) {
    case 'age': {
      // Nudge the lower bound forward a day so we never hit the reference date.
      const oneDay = 24 * 60 * 60 * 1000;
      const from =
        new Date(refDate).setUTCFullYear(refYear - max - 1) + oneDay;
      const to = new Date(refDate).setUTCFullYear(refYear - min);

      if (from > to) {
        throw new FakerError(
          `Max age ${max} should be greater than or equal to min age ${min}.`
        );
      }

      return faker.date.between({ from, to });
    }

    case 'year': {
      // Stay off Jan 1 / Dec 31 so timezone shifts don't leak into other years.
      const from = new Date(Date.UTC(0, 0, 2)).setUTCFullYear(min);
      const to = new Date(Date.UTC(0, 11, 30)).setUTCFullYear(max);

      if (from > to) {
        throw new FakerError(
          `Max year ${max} should be greater than or equal to min year ${min}.`
        );
      }

      return faker.date.between({ from, to });
    }
  }
}
