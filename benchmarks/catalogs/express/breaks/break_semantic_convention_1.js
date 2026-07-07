// Break: console.log trace statements sprinkled into a settings helper
// instead of the module's own debug() tracer (var debug =
// require('debug')('express:application') at the top of this file).
// console.log is 0-usage anywhere in lib/*.js at the pinned SHA — the only
// console.* call in lib/ is the single console.error fallback inside
// logerror(). Misuse of the repo's own attested logging construct, not a
// foreign dependency — secondary, never gated.
app.describeSetting = function describeSetting(name) {
  console.log('reading setting %s', name);
  var value = this.settings[name];
  console.log('setting %s resolved to %o', name, value);
  return value;
};
