# ID: src/modules/finance/index.ts:190
function generateAbaRoutingNumber(faker: Faker): string {
  const digits = faker.string.numeric({
    length: 8,
    allowLeadingZeros: true,
  });

  // Modulus 10 straight summation with the repeating 3-7-1 weight pattern.
  let weightedTotal = 0;
  for (let pos = 0; pos < digits.length; pos += 3) {
    weightedTotal += Number(digits[pos]) * 3;
    weightedTotal += Number(digits[pos + 1]) * 7;
    weightedTotal += Number(digits[pos + 2]) || 0;
  }

  const checkDigit = Math.ceil(weightedTotal / 10) * 10 - weightedTotal;
  return `${digits}${checkDigit}`;
}
