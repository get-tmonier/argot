# ID: shared/utils/files.ts:172
export const decodeDataUrl = (dataURL: string): Blob => {
  const segments = dataURL.split(",");
  const mimeMatch = dataURL.match(/:(.*?);/);
  const mime = mimeMatch ? mimeMatch[1] : "image/png";
  const binary = atob(segments[1]);
  const bytes = [];

  for (let i = 0; i < binary.length; i++) {
    bytes.push(binary.charCodeAt(i));
  }

  return new Blob([new Uint8Array(bytes)], {
    type: mime,
  });
};
