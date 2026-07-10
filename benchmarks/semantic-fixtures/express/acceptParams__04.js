# ID: lib/utils.js:89
function parseAcceptHeaderParams(str) {
  const length = str.length;
  const firstSemi = str.indexOf(';');
  let cursor = firstSemi === -1 ? length : firstSemi;

  const result = { value: str.slice(0, cursor).trim(), quality: 1, params: {} };

  while (cursor < length) {
    const eqIndex = str.indexOf('=', cursor);
    if (eqIndex === -1) break;

    const semiIndex = str.indexOf(';', cursor);
    const paramEnd = semiIndex === -1 ? length : semiIndex;

    // the '=' belongs to a later param — rewind to the previous ';'
    if (eqIndex > paramEnd) {
      cursor = str.lastIndexOf(';', eqIndex - 1) + 1;
      continue;
    }

    const key = str.slice(cursor, eqIndex).trim();
    const value = str.slice(eqIndex + 1, paramEnd).trim();

    if (key === 'q') {
      result.quality = parseFloat(value);
    } else {
      result.params[key] = value;
    }

    cursor = paramEnd + 1;
  }

  return result;
}
