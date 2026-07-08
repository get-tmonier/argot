# ID: lib/response.js:1026
function serializeJsonSafe(value, replacer, spaces, escape) {
  let json = replacer || spaces
    ? JSON.stringify(value, replacer, spaces)
    : JSON.stringify(value);

  if (escape && typeof json === 'string') {
    // escape characters that can trigger HTML content sniffing
    json = json.replace(/[<>&]/g, (char) => {
      switch (char.charCodeAt(0)) {
        case 0x3c: return '\\u003c';
        case 0x3e: return '\\u003e';
        case 0x26: return '\\u0026';
        default: return char;
      }
    });
  }

  return json;
}
