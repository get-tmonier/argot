# ID: lib/request.js:64
function readRequestHeader(req, name) {
  if (!name) {
    throw new TypeError('name argument is required to req.get');
  }

  if (typeof name !== 'string') {
    throw new TypeError('name must be a string to req.get');
  }

  const lc = name.toLowerCase();

  // Referrer and Referer are interchangeable
  if (lc === 'referer' || lc === 'referrer') {
    return req.headers.referrer || req.headers.referer;
  }

  return req.headers[lc];
}
