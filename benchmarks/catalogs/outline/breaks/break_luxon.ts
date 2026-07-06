import { DateTime } from "luxon";

// Break: luxon DateTime API where outline computes dates with date-fns.
export function relativeExpiry(iso: string): string {
  const dt = DateTime.fromISO(iso);
  const now = DateTime.now();
  const days = dt.diff(now, "days").days;
  return `${dt.plus({ days: 1 }).toFormat("yyyy-MM-dd")} (${days.toFixed(0)}d)`;
}
