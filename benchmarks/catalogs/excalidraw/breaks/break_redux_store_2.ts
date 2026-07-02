import { createStore, combineReducers } from "redux";

import type { AppState } from "./types";

type ThemeAction =
  | { type: "appState/setTheme"; payload: AppState["theme"] }
  | { type: "appState/toggleZenMode" };

// Break: a redux reducer + global store with switch-on-action-type, where
// this codebase keeps app state on the App class and in jotai atoms.
const appStateReducer = (
  state: Partial<AppState> = {},
  action: ThemeAction,
) => {
  switch (action.type) {
    case "appState/setTheme":
      return { ...state, theme: action.payload };
    case "appState/toggleZenMode":
      return { ...state, zenModeEnabled: !state.zenModeEnabled };
    default:
      return state;
  }
};

export const appStore = createStore(
  combineReducers({ appState: appStateReducer }),
);

export const setTheme = (theme: AppState["theme"]) => {
  appStore.dispatch({ type: "appState/setTheme", payload: theme });
};
