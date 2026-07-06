import { formatISO as isoStamp, subDays as sinceDays } from 'date-fns';

// Break: aliased date-fns import stamping a default-value description — the
// module specifier still fires the import stage; 'date-fns' is 0-usage in
// the corpus (the repo has no date-formatting dependency at all).
export function recentWindowDescription(days) {
  const from = sinceDays(new Date(), days);
  return `defaults to entries since ${isoStamp(from)}`;
}
