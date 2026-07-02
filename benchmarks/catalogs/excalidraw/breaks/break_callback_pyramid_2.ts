import type { LibraryItems } from "./types";

export const persistLibraryNested = (
  libraryItems: LibraryItems,
  adapter: {
    load: (onSuccess: (prev: LibraryItems) => void, onError: (e: Error) => void) => void;
    save: (items: LibraryItems, onSuccess: () => void, onError: (e: Error) => void) => void;
  },
  onSuccess: (saved: LibraryItems) => void,
  onError: (error: Error) => void,
) => {
  // Break: success/error callback parameters with nested continuation
  // callbacks, where library persistence in this codebase is a promise
  // queue with async/await and try/catch.
  adapter.load(
    (prevItems) => {
      const merged = [...prevItems, ...libraryItems];
      adapter.save(
        merged,
        () => {
          onSuccess(merged);
        },
        (saveError) => {
          adapter.save(
            prevItems,
            () => onError(saveError),
            () => onError(new Error("rollback failed: " + saveError.message)),
          );
        },
      );
    },
    (loadError) => {
      onError(loadError);
    },
  );
};
