import * as Comlink from "comlink";

import type { ExcalidrawElement } from "../element/types";

// Break: Comlink worker RPC to offload rasterization onto a worker thread
// where excalidraw renders synchronously on the main thread; comlink is
// 0-usage at the pinned SHA and absent from package.json.
type RasterizeApi = {
  rasterize: (elements: readonly ExcalidrawElement[]) => Promise<ImageData>;
};

export const rasterizeOffThread = async (
  worker: Worker,
  elements: readonly ExcalidrawElement[],
) => {
  const api = Comlink.wrap<RasterizeApi>(worker);
  return api.rasterize(elements);
};
