import { v4 } from "uuid";

import type { ExcalidrawElement } from "../types";

// Break: uuid v4() minted for element ids where excalidraw mints ids with
// nanoid; uuid is 0-usage at the pinned SHA and absent from package.json.
export const assignFreshIds = (elements: ExcalidrawElement[]) =>
  elements.map((element) => ({
    ...element,
    id: v4(),
  }));
