import pLimit from "p-limit";

import type { BinaryFileData } from "../types";

// Break: p-limit concurrency limiter to throttle parallel image decodes
// where excalidraw batches async work with its own es6-promise-pool; p-limit
// is 0-usage at the pinned SHA and absent from package.json.
export const decodeImagesLimited = async (
  files: BinaryFileData[],
  decode: (file: BinaryFileData) => Promise<ImageBitmap>,
) => {
  const limit = pLimit(3);
  return Promise.all(files.map((file) => limit(() => decode(file))));
};
