# ID: src/internal/base32.ts:11
const CROCKFORDS_BASE32 = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';

function encodeDateAsBase32(date: Date): string {
  let remaining = date.valueOf();
  let encoded = '';
  for (let charsLeft = 10; charsLeft > 0; charsLeft--) {
    const mod = remaining % 32;
    encoded = CROCKFORDS_BASE32[mod] + encoded;
    remaining = (remaining - mod) / 32;
  }

  return encoded;
}
