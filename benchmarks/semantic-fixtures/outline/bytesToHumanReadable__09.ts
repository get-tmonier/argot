# ID: shared/utils/files.ts:11
export function formatFileSize(bytes: number | undefined) {
  if (!bytes) {
    return "0 Bytes";
  }

  // Decimal (1000) units on macOS, binary (1024) units elsewhere
  const base = isMac ? 1000 : 1024;
  const threshold = isMac ? 1000 : 1024;

  if (bytes < threshold) {
    return bytes + " Bytes";
  }

  const units = ["Bytes", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
  const exponent = Math.floor(Math.log(bytes) / Math.log(base));
  const scaled = bytes / Math.pow(base, exponent);
  const rounded = parseFloat(scaled.toFixed(2));

  return `${rounded} ${units[exponent]}`;
}
