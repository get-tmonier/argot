import type { ImportedDataState } from "@excalidraw/excalidraw/data/types";

const BACKEND_V2_GET_URL = import.meta.env.VITE_APP_BACKEND_V2_GET_URL;

export const importFromBackendXHR = (
  id: string,
  onLoaded: (data: ImportedDataState) => void,
) => {
  // Break: raw XMLHttpRequest with onreadystatechange in a codebase whose
  // data layer is exclusively `await fetch(...)` wrappers.
  const xhr = new XMLHttpRequest();
  xhr.open("GET", `${BACKEND_V2_GET_URL}${id}`, true);
  xhr.responseType = "arraybuffer";
  xhr.onreadystatechange = () => {
    if (xhr.readyState !== XMLHttpRequest.DONE) {
      return;
    }
    if (xhr.status !== 200) {
      window.alert(`Couldn't load scene (status ${xhr.status})`);
      return;
    }
    const buffer = xhr.response as ArrayBuffer;
    const decoded = new TextDecoder("utf-8").decode(new Uint8Array(buffer));
    onLoaded(JSON.parse(decoded) as ImportedDataState);
  };
  xhr.onerror = () => {
    window.alert("Network error while loading the scene");
  };
  xhr.send(null);
};
