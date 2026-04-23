// Break: module-level Worker pool inside a pure data file.
const pool = [
  new Worker(new URL('./gen.ts', import.meta.url)),
  new Worker(new URL('./gen.ts', import.meta.url)),
  new Worker(new URL('./gen.ts', import.meta.url)),
];

export function nextWorker(i: number): Worker {
  return pool[i % pool.length]!;
}
