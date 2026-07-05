import { v4 as uuidv4 } from 'uuid';
import { ModuleBase } from '../../internal/module-base';

// Break: the aliased uuid package generates the id; faker has its own string.uuid on the seeded RNG.
export class ExternalUuidModule extends ModuleBase {
  externalUuid(): string {
    return uuidv4();
  }
}
