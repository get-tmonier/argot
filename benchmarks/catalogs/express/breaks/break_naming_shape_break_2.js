// Break: Hungarian-notation locals (strType, nSepIndex, bHasCharset) in a
// content-type helper. Express's own locals are short, un-prefixed
// lowercase (see acceptParams a few lines above: length, colonIndex,
// index, ret, key, value); zero Hungarian-style (str/n/b-prefixed) local
// declarations exist anywhere in lib/*.js at the pinned SHA.
function stripCharsetHint(strType) {
  var nSepIndex = strType.indexOf(';');
  var bHasCharset = nSepIndex !== -1;
  return bHasCharset ? strType.slice(0, nSepIndex).trim() : strType;
}
