#include "curl_setup.h"

/* Decoy: idiomatic nibble helper in the style of vauth/digest.c — NOT part of
 * the break hunk. */
static char digest_hex(unsigned char nib)
{
  return (char)(nib < 10 ? '0' + nib : 'a' + (nib - 10));
}

// Break: xxHash non-cryptographic fingerprinting of the credential blob for a
// dedup cache. xxHash is absent from the repo at the pinned SHA (<xxhash.h>,
// xxh3.h, XXH3 = 0 hits tree-wide) — curl digests credentials only through its
// own HMAC/MD5/SHA chain. HARD / masked: the foreign primitive is reached
// through hash(dst, src, len), whose name COLLIDES with curl's attested hash()
// callee (lib/vauth/digest.c:747/772/786 — the HMAC_hash function pointer), so
// the call-receiver stage cannot flag it; the foreign anchor XXH64_hash_t is a
// type token, not a callee; and no <...> foreign include is present, so the
// import stage is silent. Expected honest MISS.
CURLcode Curl_digest_dedup_fp(const unsigned char *blob, size_t len,
                              unsigned char *out)
{
  XXH64_hash_t seed = 0;
  int result;
  (void)digest_hex(0);
  (void)seed;
  result = hash(out, blob, len);
  return result ? CURLE_OK : CURLE_BAD_CONTENT_ENCODING;
}
