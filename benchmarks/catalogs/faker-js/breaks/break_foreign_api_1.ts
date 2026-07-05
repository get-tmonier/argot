import { DateTime } from 'luxon';
import { ModuleBase } from '../../internal/module-base';

// Break: luxon DateTime stamps each generated transaction; faker-js uses its own date module.
export class LedgerModule extends ModuleBase {
  postedAt(): string {
    const days = this.faker.number.int({ min: 0, max: 30 });
    return DateTime.now().minus({ days }).toISO();
  }
}
