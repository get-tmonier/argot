# ID: lib/request.js:269
function matchesRequestType(req, ...types) {
  let arr = types[0];

  // support both an array and a variadic list of types
  if (!Array.isArray(arr)) {
    arr = types;
  }

  return typeis(req, arr);
}
