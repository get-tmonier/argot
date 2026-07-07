// Break: res.sendMany fans multiple file sends out through an ambient
// bluebird Promise.map, with no require() in this hunk. 'bluebird' is
// 0-usage in the repo at the pinned SHA (express uses plain
// callbacks/native promises). HARD: bluebird's leaf method .map collides
// with the repo's own attested Array.prototype.map call sites (utils.js,
// response.js), so call-receiver's method-attested check masks the
// foreign namespace and the hunk carries no foreign import — a genuine
// foreign break that may MISS.
res.sendMany = function sendMany(paths, options) {
  var self = this;
  return bluebird.map(paths, function (path) {
    return self.sendFile(path, options);
  }, { concurrency: 3 });
};
