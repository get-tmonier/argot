# ID: lib/utils.js:130
function resolveEtagFn(val) {
  if (typeof val === 'function') {
    return val;
  }

  switch (val) {
    case false:
      return undefined;
    case true:
    case 'weak':
      return exports.wetag;
    case 'strong':
      return exports.etag;
    default:
      throw new TypeError('unknown value for etag function: ' + val);
  }
}
