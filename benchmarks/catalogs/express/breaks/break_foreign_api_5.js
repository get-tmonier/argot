// Break: req.fetchRemoteType issues an outbound GET through an ambient
// 'got' HTTP client instance, with no require() in this hunk. 'got' is
// 0-usage in the repo at the pinned SHA (express has no HTTP client of its
// own). HARD: got's leaf method .get collides with the repo's heavily
// attested req.get/res.get/app.get, so call-receiver's method-attested
// check masks the foreign namespace and the hunk carries no foreign
// import — a genuine foreign break that may MISS.
req.fetchRemoteType = function fetchRemoteType(url) {
  return got.get(url, { responseType: 'json' }).then(function (response) {
    return response.body.contentType;
  });
};
