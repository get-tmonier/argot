import { produce } from "immer";

import type { AppState } from "../types";

// Break: immer produce() for immutable appState updates where excalidraw
// derives next state with plain object spread; immer is 0-usage at the
// pinned SHA and absent from package.json.
export const toggleGridMode = (appState: AppState): AppState =>
  produce(appState, (draft) => {
    draft.gridModeEnabled = !draft.gridModeEnabled;
    draft.scrolledOutside = false;
  });
