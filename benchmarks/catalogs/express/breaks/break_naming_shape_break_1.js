var debug = require('debug')('express:request');

// Break: snake_case identifiers (get_forwarded_chain, proxy_hops) in an
// otherwise strictly camelCase file. Express's own naming is uniformly
// camelCase throughout lib/*.js (req.acceptsCharsets, req.acceptsLanguages,
// defineGetter callbacks like subdomains/hostname), with a single
// incidental exception at the pinned SHA (the inner closure function
// `mounted_app` in application.js's app.use). Verified: zero other
// snake_case function or variable declarations across lib/*.js.
function get_forwarded_chain(req) {
  var proxy_hops = req.headers['x-forwarded-for'];
  return proxy_hops ? proxy_hops.split(',').map(function (hop) {
    return hop.trim();
  }) : [];
}
