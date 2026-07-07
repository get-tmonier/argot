# ID: server/utils/url.ts:97
export async function assertUrlIsPublic(url: string) {
  // URL.hostname keeps the brackets around IPv6 literals; net.isIP rejects
  // them, so strip the brackets before checking.
  const hostname = new URL(url).hostname.replace(/^\[|\]$/g, "");

  if (net.isIP(hostname)) {
    if (isPrivateIP(hostname) && !isAllowedPrivateIP(hostname)) {
      throw InvalidRequestError(
        `DNS lookup ${hostname} is not allowed.` +
          (env.isCloudHosted
            ? ""
            : " To allow this request, add the IP address or CIDR range to the ALLOWED_PRIVATE_IP_ADDRESSES environment variable.")
      );
    }
    return;
  }

  const { address } = await dns.promises.lookup(hostname);
  if (isPrivateIP(address) && !isAllowedPrivateIP(address)) {
    throw InvalidRequestError(
      `DNS lookup ${address} (${hostname}) is not allowed.` +
        (env.isCloudHosted
          ? ""
          : " To allow this request, add the IP address or CIDR range to the ALLOWED_PRIVATE_IP_ADDRESSES environment variable.")
    );
  }
}
