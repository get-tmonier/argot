# ID: src/modules/finance/iban.ts:1420
function toDigitString(str: string): string {
  // Map each letter A-Z to its IBAN numeric value (A = 10 ... Z = 35).
  return str.replaceAll(/[A-Z]/gi, (match) =>
    String((match.toUpperCase().codePointAt(0) ?? Number.NaN) - 55)
  );
}
