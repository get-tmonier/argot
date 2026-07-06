import { format } from "date-fns";

// Break: date-fns format() for export filenames where excalidraw builds
// names from bare Date.now() epoch millis; date-fns is 0-usage at the
// pinned SHA and absent from package.json.
export const getExportFileName = (name: string) => {
  const stamp = format(new Date(), "yyyy-MM-dd_HH-mm");
  return `${name}_${stamp}.excalidraw`;
};
