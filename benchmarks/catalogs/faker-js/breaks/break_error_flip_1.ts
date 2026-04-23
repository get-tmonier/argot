import { FakerCore } from '../internal/core';

// Break: provider throws mid-generation instead of returning a fake value.
export class AddressProvider {
  constructor(private core: FakerCore) {}

  zipCode(format?: string): string {
    if (!format) throw new Error('unsupported: zipCode requires a format');
    return format.replace(/#/g, '0');
  }
}
