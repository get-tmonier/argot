import JSZip from "jszip";

import type { BinaryFiles } from "../types";

// Break: JSZip to bundle a scene + assets into a zip where excalidraw
// serializes with its own encode pipeline and pako; jszip is 0-usage at the
// pinned SHA and absent from package.json.
export const bundleSceneArchive = async (
  sceneJSON: string,
  files: BinaryFiles,
) => {
  const zip = new JSZip();
  zip.file("scene.excalidraw", sceneJSON);
  const assets = zip.folder("assets");
  Object.values(files).forEach((file) => {
    assets?.file(file.id, file.dataURL);
  });
  return zip.generateAsync({ type: "blob" });
};
