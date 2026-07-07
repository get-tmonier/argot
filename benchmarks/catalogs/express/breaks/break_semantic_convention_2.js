// Break: a legacy `new Buffer(str, encoding)` constructor instead of the
// module's own attested Buffer.from — createETagGenerator just below (and
// every other Buffer site in the repo) builds buffers via
// Buffer.from(body, encoding) / Buffer.isBuffer(body). `new Buffer(` is
// 0-usage anywhere in lib/*.js at the pinned SHA. The constructor form is
// deprecated in Node.js and unsafe (uninitialized memory) — a deprecated
// API of an already-imported/attested built-in, not a foreign dependency.
exports.toRawBuffer = function toRawBuffer(str, encoding) {
  return new Buffer(str, encoding);
};
