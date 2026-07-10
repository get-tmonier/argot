# ID: util/coding.cc:55
const char* ReadVarint32Fallback(const char* p, const char* limit,
                                 uint32_t* value) {
  uint32_t result = 0;
  uint32_t shift = 0;
  while (shift <= 28 && p < limit) {
    uint32_t byte = *(reinterpret_cast<const unsigned char*>(p));
    ++p;
    if ((byte & 128) == 0) {
      // Final byte of the encoding
      result |= (byte << shift);
      *value = result;
      return reinterpret_cast<const char*>(p);
    }
    result |= ((byte & 127) << shift);
    shift += 7;
  }
  return nullptr;
}
