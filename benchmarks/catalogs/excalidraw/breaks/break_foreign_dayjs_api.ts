import dayjs from "dayjs";
import relativeTime from "dayjs/plugin/relativeTime";

dayjs.extend(relativeTime);

// Break: dayjs + relativeTime plugin for a "last saved" label where
// excalidraw compares bare epoch millis; dayjs is 0-usage at the pinned SHA
// and absent from package.json.
export const describeLastSaved = (savedAt: number) => {
  const when = dayjs(savedAt);
  return `Saved ${when.fromNow()} (${when.format("HH:mm")})`;
};
