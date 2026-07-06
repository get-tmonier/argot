import dayjs from 'dayjs';
import { ModuleBase } from '../../internal/module-base';

// Break: dayjs date library formats a generated date; faker-js ships no date lib.
export class RelativeDateModule extends ModuleBase {
  isoWeekday(offsetDays = 0): string {
    const base = this.faker.number.int({ min: 0, max: 6 });
    return dayjs().add(base + offsetDays, 'day').format('dddd');
  }
}
