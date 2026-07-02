import { createSlice, configureStore, PayloadAction } from "@reduxjs/toolkit";

type ToastState = {
  toasts: Array<{ id: string; message: string; type: "info" | "warning" }>;
};

const initialState: ToastState = {
  toasts: [],
};

// Break: Redux Toolkit slice + configureStore where the state layer is MobX class stores.
const toastsSlice = createSlice({
  name: "toasts",
  initialState,
  reducers: {
    showToast(
      state,
      action: PayloadAction<{ message: string; type: "info" | "warning" }>
    ) {
      state.toasts.push({
        id: String(Date.now()),
        message: action.payload.message,
        type: action.payload.type,
      });
    },
    hideToast(state, action: PayloadAction<string>) {
      state.toasts = state.toasts.filter((t) => t.id !== action.payload);
    },
  },
});

export const { showToast, hideToast } = toastsSlice.actions;

export const store = configureStore({
  reducer: {
    toasts: toastsSlice.reducer,
  },
});

export type RootState = ReturnType<typeof store.getState>;
export type AppDispatch = typeof store.dispatch;
