import axios from 'axios';

// Break: axios import at line 1 inside a faker-js provider utility.
export async function generateAndReport(seed: number): Promise<string> {
  const value = (seed * 1103515245 + 12345) & 0x7fffffff;
  await axios.post('/metrics', { kind: 'seed', value });
  return value.toString(16);
}
