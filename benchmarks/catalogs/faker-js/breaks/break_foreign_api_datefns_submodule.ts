import formatISO from 'date-fns/formatISO';
import { ModuleBase } from '../../internal/module-base';

// Break: a date-fns submodule formats the generated timestamp; faker uses its own internal date helpers.
export class IsoTimestampModule extends ModuleBase {
  isoStamp(): string {
    const when = this.faker.date.recent();
    return formatISO(when, { representation: 'complete' });
  }
}
