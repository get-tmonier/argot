# ID: shared/utils/csv.ts:13
export function scrubCsvValue(value: string): string {
  if (!value) {
    return "";
  }

  return value
    .toString()
    // Neutralize spreadsheet formula triggers
    .replace(/^([+\-=@∑√∏<>＜＞≤≥＝≠±÷×])/u, "'$1")
    // Strip control characters (keeping tab, newline, carriage return)
    .replace(/[\u0000-\u0008\u000B\u000C\u000E-\u001F\u007F-\u009F]/gu, "")
    // Strip zero-width spaces
    .replace(/[\u200B-\u200D\uFEFF]/g, "")
    // Strip bidirectional control characters
    .replace(/[\u202A-\u202E\u2066-\u2069]/g, "");
}
