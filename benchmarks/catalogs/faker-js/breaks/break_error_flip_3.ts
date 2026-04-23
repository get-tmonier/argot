import { FakerModule } from '../internal/core';

// Break: seed() override throws when given a non-number, breaking determinism contract.
export class SeededModule extends FakerModule {
  seed(value?: number | number[]): void {
    if (value === undefined) throw new Error('seed() requires a numeric seed');
    super.seed(value);
  }
}
