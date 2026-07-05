// Break: zustand create() global store instead of Dagit's Recoil atoms / useQueryPersistedState.
// Dagit holds shared UI state in Recoil atoms (useRecoilState/useRecoilValue) under RecoilRoot, and
// URL-synced state in useQueryPersistedState. A zustand create() store with a generated useStore hook is a
// different global-state library that never appears in ui-core or app-oss.
import {create} from 'zustand';

interface RunFilterState {
  statuses: string[];
  pipelineName: string | null;
  setStatuses: (statuses: string[]) => void;
  setPipelineName: (name: string | null) => void;
  reset: () => void;
}

export const useRunFilterStore = create<RunFilterState>((set) => ({
  statuses: [],
  pipelineName: null,
  setStatuses: (statuses) => set({statuses}),
  setPipelineName: (pipelineName) => set({pipelineName}),
  reset: () => set({statuses: [], pipelineName: null}),
}));
