# ID: lib/urlapi.c:469
/* Validate the syntax of a URL hostname component. */
static CURLUcode check_host_syntax(struct Curl_URL *u, char *hostname,
                                   size_t namelen)
{
  size_t goodlen;
  DEBUGASSERT(hostname);

  if(!namelen)
    return CURLUE_NO_HOST;

  /* a bracketed host is an IPv6 literal, hand it to the IPv6 parser */
  if(hostname[0] == '[')
    return ipv6_parse(u, hostname, namelen);

  /* the whole name must be free of characters that cannot occur in a host */
  goodlen = strcspn(hostname, HOSTNAME_INVALID_CHARS);
  if(goodlen != namelen)
    return CURLUE_BAD_HOSTNAME;

  /* a single dot on its own is not a valid host */
  if((namelen == 1) && (hostname[0] == '.'))
    return CURLUE_BAD_HOSTNAME;

  /* more than one trailing dot is rejected */
  if((namelen >= 2) &&
     (hostname[namelen - 1] == '.') && (hostname[namelen - 2] == '.'))
    return CURLUE_BAD_HOSTNAME;

  return CURLUE_OK;
}
