# ID: lib/response.js:323
function replyWithStatusCode(res, statusCode) {
  const body = statuses.message[statusCode] || String(statusCode);

  res.status(statusCode);
  res.type('txt');

  return res.send(body);
}
