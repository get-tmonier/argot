# ID: lib/request.js:469
function checkRequestFresh(req) {
  const method = req.method;
  const res = req.res;
  const status = res.statusCode;

  // only GET/HEAD are subject to weak freshness validation
  if (method !== 'GET' && method !== 'HEAD') {
    return false;
  }

  // 2xx or 304 as per rfc2616 14.26
  if ((status >= 200 && status < 300) || status === 304) {
    return fresh(req.headers, {
      'etag': res.get('ETag'),
      'last-modified': res.get('Last-Modified')
    });
  }

  return false;
}
