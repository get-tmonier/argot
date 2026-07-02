import type { RestoredDataState } from "./restore";

type Callback<T> = (error: Error | null, result?: T) => void;

export const loadSceneFromBlobNested = (
  blob: Blob,
  done: Callback<RestoredDataState>,
) => {
  // Break: error-first callback pyramid for blob → text → parse → restore,
  // where every load path in this codebase is flat async/await with thrown
  // errors.
  const reader = new FileReader();
  reader.onload = () => {
    const contents = reader.result;
    if (typeof contents !== "string") {
      done(new Error("couldn't read blob contents"));
      return;
    }
    parseContents(contents, (parseError, data) => {
      if (parseError) {
        done(parseError);
        return;
      }
      restoreParsed(data, (restoreError, restored) => {
        if (restoreError) {
          done(restoreError);
          return;
        }
        done(null, restored);
      });
    });
  };
  reader.onerror = () => {
    done(new Error("failed reading the blob"));
  };
  reader.readAsText(blob, "utf8");
};

const parseContents = (contents: string, cb: Callback<unknown>) => {
  try {
    cb(null, JSON.parse(contents));
  } catch (error) {
    cb(error as Error);
  }
};

const restoreParsed = (data: unknown, cb: Callback<RestoredDataState>) => {
  if (!data || typeof data !== "object") {
    cb(new Error("invalid scene data"));
    return;
  }
  cb(null, data as RestoredDataState);
};
