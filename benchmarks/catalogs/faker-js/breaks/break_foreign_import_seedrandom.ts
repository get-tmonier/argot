import seedrandom from 'seedrandom';

// Break: seedrandom drives the draw instead of the repo's own Mersenne Twister; faker ships its own seeded RNG.
export function reseededSequence(seed: string, count: number): number[] {
  const rng = seedrandom(seed);
  const out: number[] = [];
  for (let i = 0; i < count; i++) {
    out.push((rng() * 0xffffffff) >>> 0);
  }
  return out;
}
