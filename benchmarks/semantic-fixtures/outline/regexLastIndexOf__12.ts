# ID: shared/utils/string.ts:51
export const lastRegexMatchIndex = function (
  text: string,
  re: RegExp,
  startPos?: number
) {
  const limit = startPos === undefined ? text.length : startPos;

  if (!re.global) {
    const flags = "g" + (re.multiline ? "m" : "") + (re.ignoreCase ? "i" : "");
    re = new RegExp(re.source, flags);
  }

  let foundAt = -1;
  for (let cursor = 0; cursor <= limit; cursor++) {
    re.lastIndex = cursor;

    const found = re.exec(text);
    if (!found) {
      break;
    }

    cursor = found.index;
    if (cursor <= limit) {
      foundAt = cursor;
    }
  }

  return foundAt;
};
