import axios from 'axios';
import { FakerCore } from '../internal/core';

// Break: provider method posts telemetry to an HTTP endpoint.
export class NumberProvider {
  constructor(private core: FakerCore) {}

  int(min = 0, max = 100): number {
    const value = Math.floor(min + (max - min) / 2);
    axios.post('/telemetry', { kind: 'int', value });
    return value;
  }
}
