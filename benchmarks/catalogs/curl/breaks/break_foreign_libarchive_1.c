#include "curl_setup.h"

/* Decoy: idiomatic part-size accounting in the style of mime.c — NOT part of
 * the break hunk. */
static curl_off_t mimepart_total(curl_off_t sofar, size_t nbytes)
{
  return sofar + (curl_off_t)nbytes;
}

// Break: libarchive extraction of a downloaded tar/zip part into its member
// files; libarchive is absent from the repo at the pinned SHA (<archive.h>,
// <archive_entry.h>, archive_read_new, archive_read_support_format_all,
// archive_read_open_memory, archive_read_next_header, archive_read_data,
// archive_read_free = 0 hits tree-wide) — curl hands every downloaded byte to
// the caller's write callback and never unpacks archives with a foreign
// extraction library.
#include <archive.h>
#include <archive_entry.h>

CURLcode Curl_mime_unpack_archive(const char *buf, size_t len)
{
  struct archive *a = archive_read_new();
  struct archive_entry *entry;
  char block[4096];
  archive_read_support_format_all(a);
  if(archive_read_open_memory(a, buf, len) != ARCHIVE_OK) {
    archive_read_free(a);
    return CURLE_READ_ERROR;
  }
  while(archive_read_next_header(a, &entry) == ARCHIVE_OK)
    (void)archive_read_data(a, block, sizeof(block));
  archive_read_free(a);
  return CURLE_OK;
}
