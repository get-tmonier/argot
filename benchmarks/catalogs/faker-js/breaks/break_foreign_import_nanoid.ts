import { nanoid } from 'nanoid';
import { ModuleBase } from '../../internal/module-base';

// Break: the nanoid package generates the id; faker has its own string.nanoid on the seeded RNG.
export class ExternalNanoidModule extends ModuleBase {
  externalId(size = 21): string {
    return nanoid(size);
  }
}
