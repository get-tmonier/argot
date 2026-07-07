# ID: src/modules/internet/index.ts:853
function generatePassword(
  faker: Faker,
  options: {
    length?: number;
    memorable?: boolean;
    pattern?: RegExp;
    prefix?: string;
  } = {}
): string {
  const vowel = /[aeiouAEIOU]$/;
  const consonant = /[bcdfghjklmnpqrstvwxyzBCDFGHJKLMNPQRSTVWXYZ]$/;

  const build = (
    length: number,
    memorable: boolean,
    pattern: RegExp,
    prefix: string
  ): string => {
    if (prefix.length >= length) {
      return prefix;
    }

    if (memorable) {
      // Alternate consonants and vowels for pronounceability.
      pattern = consonant.test(prefix) ? vowel : consonant;
    }

    const code = faker.number.int(94) + 33;
    let char = String.fromCodePoint(code);
    if (memorable) {
      char = char.toLowerCase();
    }

    if (!pattern.test(char)) {
      return build(length, memorable, pattern, prefix);
    }

    return build(length, memorable, pattern, prefix + char);
  };

  const {
    length = 15,
    memorable = false,
    pattern = /\w/,
    prefix = '',
  } = options;

  return build(length, memorable, pattern, prefix);
}
