# ID: src/modules/internet/index.ts:310
declare const charMapping: Record<string, string>;

function buildUsername(
  faker: Faker,
  options: { firstName?: string; lastName?: string } = {}
): string {
  const {
    firstName = faker.person.firstName(),
    lastName = faker.person.lastName(),
    lastName: providedLastName,
  } = options;

  const joiner = faker.helpers.arrayElement(['.', '_']);
  const disambiguator = faker.number.int(99);
  const strategies: Array<() => string> = [
    () => `${firstName}${joiner}${lastName}${disambiguator}`,
    () => `${firstName}${joiner}${lastName}`,
  ];
  if (!providedLastName) {
    strategies.push(() => `${firstName}${disambiguator}`);
  }

  let result = faker.helpers.arrayElement(strategies)();

  // Strip accents: decompose to base char + combining mark, then drop the marks.
  result = result.normalize('NFKD').replaceAll(/[̀-ͯ]/g, '');

  result = [...result]
    .map((char) => {
      // Use a transliteration mapping for Cyrillic, Greek, etc. when available.
      if (charMapping[char]) {
        return charMapping[char];
      }

      const charCode = char.codePointAt(0) ?? Number.NaN;
      if (charCode < 0x80) {
        // Plain ASCII passes through untouched.
        return char;
      }

      // Everything else falls back to its base-36 code point.
      return charCode.toString(36);
    })
    .join('');

  result = result.replaceAll("'", '');
  result = result.replaceAll(' ', '');

  return result;
}
