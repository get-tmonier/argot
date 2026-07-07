# ID: src/modules/helpers/index.ts:244
function replaceSymbols(faker: Faker, template: string = ''): string {
  const alpha = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('');
  let result = '';

  for (let i = 0; i < template.length; i++) {
    const current = template.charAt(i);
    if (current === '#') {
      // digit 0-9
      result += faker.number.int(9);
    } else if (current === '?') {
      // uppercase letter A-Z
      result += faker.helpers.arrayElement(alpha);
    } else if (current === '*') {
      // either a digit or an uppercase letter
      result += faker.datatype.boolean()
        ? faker.helpers.arrayElement(alpha)
        : faker.number.int(9);
    } else {
      result += current;
    }
  }

  return result;
}
