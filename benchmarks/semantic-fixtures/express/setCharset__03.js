# ID: lib/utils.js:225
function applyCharset(type, charset) {
  if (!type || !charset) {
    return type;
  }

  // parse the Content-Type, inject the charset parameter, re-serialize
  const parsed = contentType.parse(type);
  parsed.parameters.charset = charset;

  return contentType.format(parsed);
}
