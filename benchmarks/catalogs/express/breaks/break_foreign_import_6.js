var dayjs = require('dayjs');

// Break: formatHttpDate stringifies a Date via dayjs instead of the
// native Date the rest of the codebase relies on (see res.cookie's
// `new Date(Date.now() + maxAge)` in response.js). 'dayjs' is 0-usage in
// the repo at the pinned SHA.
exports.formatHttpDate = function formatHttpDate(date) {
  return dayjs(date).utc().format('ddd, DD MMM YYYY HH:mm:ss [GMT]');
};
