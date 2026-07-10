# ID: lib/application.js:351
function applySetting(app, setting, val) {
  // getter form: only the setting name was provided
  if (arguments.length === 2) {
    return app.settings[setting];
  }

  debug('set "%s" to %o', setting, val);
  app.settings[setting] = val;

  // recompile derived settings when their source changes
  switch (setting) {
    case 'etag':
      app.set('etag fn', compileETag(val));
      break;
    case 'query parser':
      app.set('query parser fn', compileQueryParser(val));
      break;
    case 'trust proxy':
      app.set('trust proxy fn', compileTrust(val));

      // trust proxy inherit back-compat
      Object.defineProperty(app.settings, trustProxyDefaultSymbol, {
        configurable: true,
        value: false
      });
      break;
  }

  return app;
}
