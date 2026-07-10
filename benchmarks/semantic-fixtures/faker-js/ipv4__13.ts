# ID: src/modules/internet/index.ts:673
declare const ipv4Networks: Record<string, string>;

function generateIpv4(
  faker: Faker,
  options: { cidrBlock?: string; network?: string } = {}
): string {
  const { network = 'any', cidrBlock = ipv4Networks[network] } = options;

  if (!/^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\/\d{1,2}$/.test(cidrBlock)) {
    throw new FakerError(
      `Invalid CIDR block provided: ${cidrBlock}. Must be in the format x.x.x.x/y.`
    );
  }

  const [baseText, prefixBits] = cidrBlock.split('/');
  const hostMask = 0xffffffff >>> Number.parseInt(prefixBits);
  const [oct1, oct2, oct3, oct4] = baseText.split('.').map(Number);
  const baseInt = (oct1 << 24) | (oct2 << 16) | (oct3 << 8) | oct4;
  const networkInt = baseInt & ~hostMask;
  const hostOffset = faker.number.int(hostMask);
  const addressInt = networkInt | hostOffset;
  return [
    (addressInt >>> 24) & 0xff,
    (addressInt >>> 16) & 0xff,
    (addressInt >>> 8) & 0xff,
    addressInt & 0xff,
  ].join('.');
}
