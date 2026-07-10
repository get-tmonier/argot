# ID: shared/utils/currency.ts:102
export function toNumericAmount(value: string): number | null {
  if (!value || value.trim().length === 0) {
    return null;
  }

  const trimmed = value.trim();

  // Parentheses or a leading/trailing minus signal an accounting negative
  const isNegative =
    trimmed.startsWith("(") ||
    trimmed.startsWith("-") ||
    trimmed.includes(")-") ||
    (trimmed.endsWith(")") && trimmed.includes("("));

  // Strip currency symbols (longest first so multi-char symbols like R$ go first)
  let cleaned = trimmed;
  const byLength = [...currencySymbols].sort((a, b) => b.length - a.length);
  for (const symbol of byLength) {
    cleaned = cleaned.split(symbol).join("");
  }
  cleaned = cleaned
    .replace(/[()]/g, "")
    .replace(/\s/g, "")
    .replace(/^-|-$/g, "");

  // The separator appearing last is the decimal separator
  const lastComma = cleaned.lastIndexOf(",");
  const lastPeriod = cleaned.lastIndexOf(".");
  const hasComma = lastComma !== -1;
  const hasPeriod = lastPeriod !== -1;

  if (hasComma && hasPeriod) {
    if (lastComma > lastPeriod) {
      // European style: comma is decimal, period is thousands
      cleaned = cleaned.replace(/\./g, "").replace(",", ".");
    } else {
      // US/UK style: period is decimal, comma is thousands
      cleaned = cleaned.replace(/,/g, "");
    }
  } else if (hasComma) {
    const parts = cleaned.split(",");
    if (parts.length === 2 && parts[1].length <= 2) {
      cleaned = cleaned.replace(",", ".");
    } else {
      cleaned = cleaned.replace(/,/g, "");
    }
  } else if (hasPeriod) {
    const parts = cleaned.split(".");
    if (!(parts.length === 2 && parts[1].length <= 2)) {
      cleaned = cleaned.replace(/\./g, "");
    }
  }

  const amount = parseFloat(cleaned);
  if (isNaN(amount)) {
    return null;
  }

  return isNegative ? -Math.abs(amount) : amount;
}
