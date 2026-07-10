# ID: util/coding.h:163
char* PackVarint64(char* dst, uint64_t value) {
  static const unsigned int kHighBit = 128;
  unsigned char* ptr = lossless_cast<unsigned char*>(dst);
  while (value >= kHighBit) {
    *(ptr++) = (value & (kHighBit - 1)) | kHighBit;
    value >>= 7;
  }
  *(ptr++) = static_cast<unsigned char>(value);
  return lossless_cast<char*>(ptr);
}
