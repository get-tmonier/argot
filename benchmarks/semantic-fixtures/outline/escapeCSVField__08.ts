# ID: shared/utils/csv.ts:38
export function quoteCsvField(value: unknown): string {
  if (value === null || value === undefined) {
    return "";
  }

  const asString =
    typeof value === "object"
      ? JSON.stringify(value)
      : String(value as string | number | boolean);

  // Wrap in quotes when the field contains a delimiter, quote, or newline
  const needsQuoting =
    asString.includes(",") ||
    asString.includes('"') ||
    asString.includes("\n");

  if (needsQuoting) {
    return `"${asString.replace(/"/g, '""')}"`;
  }

  return asString;
}
