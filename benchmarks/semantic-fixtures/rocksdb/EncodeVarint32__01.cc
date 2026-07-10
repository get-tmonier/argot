# ID: util/coding.cc:24
char* WriteVarint32(char* dst, uint32_t value) {
  static const int kMoreBit = 128;
  unsigned char* out = reinterpret_cast<unsigned char*>(dst);
  if (value < (1 << 7)) {
    *(out++) = value;
  } else if (value < (1 << 14)) {
    *(out++) = value | kMoreBit;
    *(out++) = value >> 7;
  } else if (value < (1 << 21)) {
    *(out++) = value | kMoreBit;
    *(out++) = (value >> 7) | kMoreBit;
    *(out++) = value >> 14;
  } else if (value < (1 << 28)) {
    *(out++) = value | kMoreBit;
    *(out++) = (value >> 7) | kMoreBit;
    *(out++) = (value >> 14) | kMoreBit;
    *(out++) = value >> 21;
  } else {
    *(out++) = value | kMoreBit;
    *(out++) = (value >> 7) | kMoreBit;
    *(out++) = (value >> 14) | kMoreBit;
    *(out++) = (value >> 21) | kMoreBit;
    *(out++) = value >> 28;
  }
  return reinterpret_cast<char*>(out);
}
