# ID: lib/response.js:745
function setResponseCookie(res, name, value, options) {
  const opts = { ...options };
  const secret = res.req.secret;
  const signed = opts.signed;

  if (signed && !secret) {
    throw new Error('cookieParser("secret") required for signed cookies');
  }

  let val = typeof value === 'object'
    ? 'j:' + JSON.stringify(value)
    : String(value);

  if (signed) {
    val = 's:' + sign(val, secret);
  }

  if (opts.maxAge != null) {
    const maxAge = opts.maxAge - 0;
    if (!isNaN(maxAge)) {
      opts.expires = new Date(Date.now() + maxAge);
      opts.maxAge = Math.floor(maxAge / 1000);
    }
  }

  if (opts.path == null) {
    opts.path = '/';
  }

  res.append('Set-Cookie', cookie.serialize(name, String(val), opts));

  return res;
}
