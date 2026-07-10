# ID: src/modules/string/uuid.ts:24
function uuidVersion7(faker: SimpleFaker, refDate: Date): string {
  const epochMs = refDate.valueOf();
  const clampedMs = Math.max(epochMs, 0);
  const timestampHex = clampedMs.toString(16).padStart(12, '0').slice(-12);

  const timestampSection = [
    timestampHex.substring(0, 8),
    timestampHex.substring(8),
  ].join('-');

  const entropySection = '7xxx-yxxx-xxxxxxxxxxxx'
    .replaceAll('x', () => faker.number.hex({ min: 0x0, max: 0xf }))
    .replaceAll('y', () => faker.number.hex({ min: 0x8, max: 0xb }));

  return `${timestampSection}-${entropySection}`;
}
