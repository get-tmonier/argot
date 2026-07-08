# ID: lib/response.js:98
function buildLinkHeader(res, links) {
  const formatted = Object.keys(links).map((rel) => {
    const value = links[rel];
    // allow multiple links when the value is an array
    if (Array.isArray(value)) {
      return value.map((singleLink) => `<${singleLink}>; rel="${rel}"`).join(', ');
    }
    return `<${value}>; rel="${rel}"`;
  }).join(', ');

  let existing = res.get('Link') || '';
  if (existing) existing += ', ';

  return res.set('Link', existing + formatted);
}
