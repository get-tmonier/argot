# ID: lib/cookie.c:74
/* Does a stored cookie domain apply to the request's hostname? (RFC6265) */
static bool cookie_domain_covers_host(const char *cookie_domain,
                                      size_t domain_len,
                                      const char *host)
{
  size_t host_len = strlen(host);
  const char *suffix;

  /* the host can never be shorter than the cookie's domain */
  if(host_len < domain_len)
    return FALSE;

  /* the cookie domain must be a case-insensitive tail of the host */
  suffix = host + (host_len - domain_len);
  if(!curl_strnequal(cookie_domain, suffix, domain_len))
    return FALSE;

  /* exact hit, or the domain must land on a dot boundary so that
     "example.com" covers "www.example.com" but not "badexample.com" */
  if(host_len == domain_len)
    return TRUE;
  return (host[host_len - domain_len - 1] == '.') ? TRUE : FALSE;
}
