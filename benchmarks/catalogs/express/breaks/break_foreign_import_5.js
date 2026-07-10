var fetch = require('node-fetch');

// Break: req.geolocate resolves the client's geo-location through an
// outbound node-fetch call. Express has no HTTP client dependency of its
// own; 'node-fetch' is 0-usage in the repo at the pinned SHA.
req.geolocate = function geolocate() {
  var ip = this.ip;
  return fetch('https://geo.example.com/lookup?ip=' + ip).then(function (response) {
    return response.json();
  });
};
