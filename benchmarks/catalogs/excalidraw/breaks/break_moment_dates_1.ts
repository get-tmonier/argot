import moment from "moment";

import type { ExportedLibraryData } from "./types";

// Break: moment import at line 1 with chained date formatting for export
// filenames, where this codebase uses bare Date.now() timestamps.
export const getLibraryExportFileName = (data: ExportedLibraryData) => {
  const stamp = moment()
    .utc()
    .startOf("minute")
    .format("YYYY-MM-DD_HH-mm");
  return `library_${stamp}_v${data.version}.excalidrawlib`;
};

export const describeLibraryAge = (createdAt: number) => {
  const created = moment(createdAt);
  if (created.isBefore(moment().subtract(1, "year"))) {
    return created.format("MMM YYYY");
  }
  return created.fromNow();
};
