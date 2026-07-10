# ID: server/utils/url.ts:59
function ipInAllowList(ip: string): boolean {
  const allowList = env.ALLOWED_PRIVATE_IP_ADDRESSES;
  if (!allowList || allowList.length === 0) {
    return false;
  }

  if (!ipaddr.isValid(ip)) {
    return false;
  }

  const addr = ipaddr.parse(ip);

  for (const entry of allowList) {
    if (net.isIP(entry)) {
      if (entry === ip) {
        return true;
      }
    } else if (ipaddr.isValid(entry.split("/")[0])) {
      try {
        if (addr.match(ipaddr.parseCIDR(entry))) {
          return true;
        }
      } catch {
        // Skip malformed CIDR entries
      }
    }
  }

  return false;
}
