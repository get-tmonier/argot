# ID: src/modules/finance/index.ts:514
function makeCreditCardNumber(
  faker: Faker,
  options: string | { issuer?: string } = {}
): string {
  if (typeof options === 'string') {
    options = { issuer: options };
  }

  const { issuer = '' } = options;

  let schema: string;
  const knownFormats = faker.definitions.finance.credit_card;
  const issuerKey = issuer.toLowerCase();
  if (issuerKey in knownFormats) {
    schema = faker.helpers.arrayElement(knownFormats[issuerKey]);
  } else if (issuer.includes('#')) {
    // The caller supplied their own template scheme.
    schema = issuer;
  } else {
    // No issuer given: pick a random one from the locale's format object.
    const candidateFormats = faker.helpers.objectValue(knownFormats);
    schema = faker.helpers.arrayElement(candidateFormats);
  }

  schema = schema.replaceAll('/', '');
  return faker.helpers.replaceCreditCardSymbols(schema);
}
