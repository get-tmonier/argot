# ID: src/modules/finance/iban.ts:1410
function mod97(digitStr: string): number {
  // Piecewise modulo-97 over a long digit string (avoids BigInt overflow).
  let remainder = 0;
  for (const element of digitStr) {
    remainder = (remainder * 10 + +element) % 97;
  }

  return remainder;
}
