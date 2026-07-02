import type { BinaryFileData, FileId } from "@excalidraw/excalidraw/types";

export const uploadFileWithProgress = (
  prefix: string,
  file: BinaryFileData,
  onProgress: (percent: number) => void,
  onDone: (fileId: FileId | null) => void,
) => {
  // Break: XMLHttpRequest upload with progress events and status polling,
  // where every persistence path in this codebase is an async/await fetch
  // wrapper returning typed results.
  const payload = new Blob([file.dataURL], { type: file.mimeType });
  const xhr = new XMLHttpRequest();
  xhr.upload.addEventListener("progress", (event) => {
    if (event.lengthComputable) {
      onProgress(Math.round((event.loaded / event.total) * 100));
    }
  });
  xhr.addEventListener("load", () => {
    if (xhr.status >= 200 && xhr.status < 300) {
      onDone(file.id);
    } else if (xhr.status === 413) {
      window.alert("File too large to upload");
      onDone(null);
    } else {
      onDone(null);
    }
  });
  xhr.addEventListener("error", () => {
    onDone(null);
  });
  xhr.open("PUT", `${prefix}/${file.id}`);
  xhr.setRequestHeader("Content-Type", file.mimeType);
  xhr.send(payload);
};
