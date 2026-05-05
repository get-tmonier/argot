// Break: Redux Toolkit createSlice + configureStore for local UI filter state instead of Dagit's Recoil atoms.
// Dagit persists UI state via Recoil atoms and useQueryPersistedState (URL-synced hook).
// Redux Toolkit's createSlice, configureStore, useDispatch, and useSelector are not used in the codebase.

import {PayloadAction, configureStore, createSlice} from '@reduxjs/toolkit';
import {useDispatch, useSelector} from 'react-redux';

interface PipelineFilterState {
  showFailed: boolean;
  showPending: boolean;
  selectedRepo: string | null;
}

const initialState: PipelineFilterState = {
  showFailed: true,
  showPending: true,
  selectedRepo: null,
};

const pipelineFilterSlice = createSlice({
  name: 'pipelineFilter',
  initialState,
  reducers: {
    toggleFailed(state) {
      state.showFailed = !state.showFailed;
    },
    togglePending(state) {
      state.showPending = !state.showPending;
    },
    setRepo(state, action: PayloadAction<string | null>) {
      state.selectedRepo = action.payload;
    },
  },
});

export const pipelineFilterStore = configureStore({
  reducer: {pipelineFilter: pipelineFilterSlice.reducer},
});

type RootState = ReturnType<typeof pipelineFilterStore.getState>;

export const {toggleFailed, togglePending, setRepo} = pipelineFilterSlice.actions;

export function usePipelineFilterState() {
  const dispatch = useDispatch<typeof pipelineFilterStore.dispatch>();
  const state = useSelector((s: RootState) => s.pipelineFilter);
  return {state, dispatch};
}
