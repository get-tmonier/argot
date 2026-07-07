# ID: src/modules/string/index.ts:174
const LOWER_CHARS = 'abcdefghijklmnopqrstuvwxyz'.split('');
const UPPER_CHARS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('');

function generateAlpha(
  faker: Faker,
  options:
    | number
    | {
        length?: number | { min: number; max: number };
        casing?: 'upper' | 'lower' | 'mixed';
        exclude?: ReadonlyArray<string> | string;
      } = {}
): string {
  if (typeof options === 'number') {
    options = { length: options };
  }

  const length = faker.helpers.rangeToNumber(options.length ?? 1);
  if (length <= 0) {
    return '';
  }

  const { casing = 'mixed' } = options;
  let { exclude = [] } = options;
  if (typeof exclude === 'string') {
    exclude = [...exclude];
  }

  let pool: string[];
  switch (casing) {
    case 'upper': {
      pool = [...UPPER_CHARS];
      break;
    }

    case 'lower': {
      pool = [...LOWER_CHARS];
      break;
    }

    case 'mixed': {
      pool = [...LOWER_CHARS, ...UPPER_CHARS];
      break;
    }
  }

  pool = pool.filter((ch) => !exclude.includes(ch));

  return faker.string.fromCharacters(pool, length);
}
