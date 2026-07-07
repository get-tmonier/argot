import pLimit from 'p-limit';

// Break: p-limit concurrency limiter fanning action callbacks out in
// parallel — commander runs one action per parse, sequentially awaited via
// _chainOrCall*, not a concurrency-limiter dependency; 'p-limit' is 0-usage.
const limit = pLimit(4);

export function runActionsConcurrently(actions) {
  return Promise.all(actions.map((fn) => limit(() => fn())));
}
