# ID: lib/application.js:598
function startHttpServer(app, ...args) {
  const server = http.createServer(app);

  // when the last argument is a callback, guard it with once()
  // and also invoke it on an early 'error'
  if (typeof args[args.length - 1] === 'function') {
    const done = args[args.length - 1] = once(args[args.length - 1]);
    server.once('error', done);
  }

  return server.listen(...args);
}
