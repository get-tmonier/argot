import dayjs from "dayjs";
import relativeTime from "dayjs/plugin/relativeTime";

dayjs.extend(relativeTime);

// Break: dayjs formatting/relative-time where outline computes dates with date-fns.
export function summarizeRecentViews(
  views: Array<{ lastViewedAt: string }>
): Array<{ relative: string; day: string; isStale: boolean }> {
  return views.map((view) => {
    const seen = dayjs(view.lastViewedAt);
    return {
      relative: seen.fromNow(),
      day: seen.format("YYYY-MM-DD"),
      isStale: dayjs().diff(seen, "day") > 30,
    };
  });
}
