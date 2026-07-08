# ID: lib/request.js:418
function resolveHostHeader(req) {
  const trust = req.app.get('trust proxy fn');
  let val = req.get('X-Forwarded-Host');

  if (!val || !trust(req.socket.remoteAddress, 0)) {
    val = req.get('Host');
  } else if (val.indexOf(',') !== -1) {
    // X-Forwarded-Host may carry a list; take the first entry
    val = val.substring(0, val.indexOf(',')).trimEnd();
  }

  return val || undefined;
}
