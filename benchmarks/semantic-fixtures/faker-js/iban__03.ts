# ID: src/modules/finance/index.ts:724
function generateIban(
  faker: Faker,
  options: { formatted?: boolean; countryCode?: string } = {}
): string {
  const { countryCode, formatted = false } = options;

  const spec = countryCode
    ? iban.formats.find((entry) => entry.country === countryCode)
    : faker.helpers.arrayElement(iban.formats);

  if (!spec) {
    throw new FakerError(`Country code ${countryCode} not supported.`);
  }

  let bbanBody = '';
  let consumed = 0;
  for (const segment of spec.bban) {
    let remaining = segment.count;
    consumed += segment.count;
    while (remaining > 0) {
      if (segment.type === 'a') {
        bbanBody += faker.helpers.arrayElement(iban.alpha);
      } else if (segment.type === 'c') {
        if (faker.datatype.boolean(0.8)) {
          bbanBody += faker.number.int(9);
        } else {
          bbanBody += faker.helpers.arrayElement(iban.alpha);
        }
      } else {
        if (remaining >= 3 && faker.datatype.boolean(0.3)) {
          if (faker.datatype.boolean()) {
            bbanBody += faker.helpers.arrayElement(iban.pattern100);
            remaining -= 2;
          } else {
            bbanBody += faker.helpers.arrayElement(iban.pattern10);
            remaining--;
          }
        } else {
          bbanBody += faker.number.int(9);
        }
      }

      remaining--;
    }

    bbanBody = bbanBody.substring(0, consumed);
  }

  let checkDigits: string | number =
    98 - iban.mod97(iban.toDigitString(`${bbanBody}${spec.country}00`));

  if (checkDigits < 10) {
    checkDigits = `0${checkDigits}`;
  }

  const full = `${spec.country}${checkDigits}${bbanBody}`;
  return formatted ? prettyPrintIban(full) : full;
}
