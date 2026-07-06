var winston = require('winston');

// Break: request-dispatch tracing swapped from express's own debug()
// (var debug = require('debug')('express:application'), used throughout
// this file) for a winston logger. 'winston' is 0-usage in the repo at
// the pinned SHA.
var logger = winston.createLogger({
  level: 'info',
  transports: [new winston.transports.Console()]
});

app.traceDispatch = function traceDispatch(req) {
  logger.info('dispatching %s %s', req.method, req.url);
};
