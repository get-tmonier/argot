# ID: lib/response.js:234
function sendJsonBody(res, obj) {
  const app = res.app;
  const replacer = app.get('json replacer');
  const spaces = app.get('json spaces');
  const escape = app.get('json escape');

  const body = stringify(obj, replacer, spaces, escape);

  // default the Content-Type to JSON when unset
  if (!res.get('Content-Type')) {
    res.set('Content-Type', 'application/json');
  }

  return res.send(body);
}
