# ID: util/coding.cc:73
const char* ParseVarint64Pointer(const char* p, const char* limit,
                                 uint64_t* value) {
  uint64_t result = 0;
  for (uint32_t shift = 0; p < limit && shift <= 63; shift += 7) {
    uint64_t byte = *(reinterpret_cast<const unsigned char*>(p));
    p++;
    if (byte & 128) {
      // More bytes follow this one
      result |= ((byte & 127) << shift);
    } else {
      result |= (byte << shift);
      *value = result;
      return reinterpret_cast<const char*>(p);
    }
  }
  return nullptr;
}
