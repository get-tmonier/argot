# ID: lib/response.js:571
function negotiateResponseFormat(res, obj) {
  const req = res.req;
  const next = req.next;

  const keys = Object.keys(obj).filter((name) => name !== 'default');
  const matched = keys.length > 0 ? req.accepts(keys) : false;

  res.vary('Accept');

  if (matched) {
    res.set('Content-Type', normalizeType(matched).value);
    obj[matched](req, res, next);
  } else if (obj.default) {
    obj.default(req, res, next);
  } else {
    next(createError(406, {
      types: normalizeTypes(keys).map((entry) => entry.value)
    }));
  }

  return res;
}
