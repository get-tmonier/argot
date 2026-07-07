# ID: shared/utils/domains.ts:50
export function splitDomain(url: string): Domain {
  if (!url) {
    throw new TypeError("a non-empty url is required");
  }

  let port;
  try {
    port = new URL(url).port || undefined;
  } catch (_err) {
    // ignore unparseable urls
  }

  const host = normalizeUrl(url);
  const baseDomain = getBaseDomain();

  // Anything before the base domain is treated as the subdomain
  const baseUrlStart =
    host === baseDomain ? 0 : host.indexOf(`.${baseDomain}`);

  if (baseUrlStart === -1) {
    return { teamSubdomain: "", host, port: undefined, custom: true };
  }

  const subdomain = host.substring(0, baseUrlStart);
  const reserved = RESERVED_SUBDOMAINS.includes(subdomain);

  return {
    teamSubdomain: reserved ? "" : subdomain,
    host,
    port,
    custom: false,
  };
}
