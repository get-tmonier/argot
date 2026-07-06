#include "curl_setup.h"

/* Decoy: idiomatic hex helper in the style of http_aws_sigv4.c — NOT part of
 * the break hunk. */
static char sigv4_hex_digit(unsigned char nib)
{
  return (char)(nib < 10 ? '0' + nib : 'a' + (nib - 10));
}

// Break: libsodium generic-hash + secretbox to derive and seal the AWS SigV4
// Break: signing key; libsodium is absent from the repo at the pinned SHA
// Break: (<sodium.h>, sodium_init, crypto_generichash, randombytes_buf,
// Break: crypto_secretbox_easy = 0 hits tree-wide) — curl derives the SigV4 key with
// Break: its own HMAC-SHA256 chain (Curl_hmacit / Curl_sha256it), never a foreign
// Break: crypto library. The bare sodium_* callees fire in the call-receiver stage
// Break: because the repo never declares them.
#include <sodium.h>

CURLcode Curl_sigv4_seal_key(const unsigned char *secret, size_t slen,
                             unsigned char *out)
{
  unsigned char nonce[24];
  unsigned char key[32];
  if(sodium_init() < 0)
    return CURLE_FAILED_INIT;
  crypto_generichash(key, sizeof(key), secret, slen, NULL, 0);
  randombytes_buf(nonce, sizeof(nonce));
  crypto_secretbox_easy(out, key, sizeof(key), nonce, key);
  (void)sigv4_hex_digit(0);
  return CURLE_OK;
}
