import { BigNumber } from 'bignumber.js';
import { ModuleBase } from '../../internal/module-base';

// Break: bignumber.js accumulates a precise monetary total through a receiver variable; faker returns plain numbers/strings.
export class PreciseAmountModule extends ModuleBase {
  runningTotal(count: number): string {
    let total = new BigNumber(0);
    for (let i = 0; i < count; i++) {
      const line = this.faker.finance.amount({ min: 1, max: 500 });
      total = total.plus(new BigNumber(line));
    }
    return total.toFixed(2);
  }
}
