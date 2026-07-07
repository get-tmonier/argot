# ID: shared/utils/domains.ts:23
function extractHostname(url: string) {
  const withoutProtocol = trim(url.replace(/(https?:)?\/\//, ""));
  // Everything before the first slash is the authority
  const authority = withoutProtocol.split("/")[0];
  // Drop any userinfo prefix such as "user:pass@host"
  const at = authority.lastIndexOf("@");
  const hostWithPort = at !== -1 ? authority.substring(at + 1) : authority;
  return hostWithPort.split(/[:?]/)[0];
}
