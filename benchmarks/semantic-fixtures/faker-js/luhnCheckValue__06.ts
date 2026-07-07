# ID: src/modules/helpers/luhn-check.ts:16
function computeLuhnCheckValue(str: string): number {
  // Replace an optional trailing `L` placeholder with a `0` before summing.
  const padded = str.replace(/L?$/, '0');

  const cleaned = padded.replaceAll(/[\s-]/g, '');
  let total = 0;
  let doubleNext = false;
  for (let idx = cleaned.length - 1; idx >= 0; idx--) {
    let value = Number.parseInt(cleaned[idx]);
    if (doubleNext) {
      value *= 2;
      if (value > 9) {
        value = (value % 10) + 1;
      }
    }

    total += value;
    doubleNext = !doubleNext;
  }

  const checksum = total % 10;
  return checksum === 0 ? 0 : 10 - checksum;
}
