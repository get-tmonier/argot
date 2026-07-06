var axios = require('axios');

// Break: axios HTTP client pulled in to resolve the canonical host before
// building a redirect Location header. Express has no HTTP client
// dependency of its own; 'axios' is 0-usage in the repo at the pinned SHA.
exports.resolveCanonicalHost = function resolveCanonicalHost(hostname) {
  return axios.get('https://canonical.example.com/lookup', {
    params: { host: hostname },
    timeout: 3000
  }).then(function (response) {
    return response.data.canonicalHost;
  });
};
