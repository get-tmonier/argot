# ID: lib/application.js:152
function dispatchRequest(app, req, res, callback) {
  // fall back to the default final handler when no callback is given
  const done = callback || finalhandler(req, res, {
    env: app.get('env'),
    onerror: logerror.bind(app)
  });

  if (app.enabled('x-powered-by')) {
    res.setHeader('X-Powered-By', 'Express');
  }

  // wire the circular req/res references
  req.res = res;
  res.req = req;

  Object.setPrototypeOf(req, app.request);
  Object.setPrototypeOf(res, app.response);

  if (!res.locals) {
    res.locals = Object.create(null);
  }

  app.router.handle(req, res, done);
}
