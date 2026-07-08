# ID: util/string_util.cc:67
int FormatByteSize(uint64_t bytes, char* output, int len) {
  const uint64_t ten = 10;
  if (bytes >= ten << 40) {
    return snprintf(output, len, "%" PRIu64 "TB", bytes >> 40);
  } else if (bytes >= ten << 30) {
    return snprintf(output, len, "%" PRIu64 "GB", bytes >> 30);
  } else if (bytes >= ten << 20) {
    return snprintf(output, len, "%" PRIu64 "MB", bytes >> 20);
  } else if (bytes >= ten << 10) {
    return snprintf(output, len, "%" PRIu64 "KB", bytes >> 10);
  }
  return snprintf(output, len, "%" PRIu64 "B", bytes);
}
