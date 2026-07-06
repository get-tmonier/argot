import Chance from 'chance';
import { ModuleBase } from '../../internal/module-base';

// Break: the Chance.js generator produces the value instead of faker's own seeded provider.
export class ChanceAnimalModule extends ModuleBase {
  private chance = new Chance();

  exoticAnimal(): string {
    return this.chance.animal({ type: 'zoo' });
  }
}
