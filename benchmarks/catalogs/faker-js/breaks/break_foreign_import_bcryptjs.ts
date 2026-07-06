import bcrypt from 'bcryptjs';
import { ModuleBase } from '../../internal/module-base';

// Break: bcryptjs hashes a generated password; faker returns plaintext fakes and ships no hashing lib.
export class HashedPasswordModule extends ModuleBase {
  hashedPassword(length = 15): string {
    const plain = this.faker.internet.password({ length });
    return bcrypt.hashSync(plain, 10);
  }
}
