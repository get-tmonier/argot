import * as math from 'mathjs';
import { ModuleBase } from '../../internal/module-base';

// Break: mathjs evaluates an expression to derive the number; faker uses plain arithmetic on its seeded RNG.
export class ComputedNumberModule extends ModuleBase {
  computed(expr = '2 * (3 + 4)'): number {
    const scale = this.faker.number.int({ min: 1, max: 10 });
    return math.evaluate(`${scale} * (${expr})`);
  }
}
