# ID: util/coding.h:237
uint16_t VarintByteCount(uint64_t value) {
  uint16_t bytes = 1;
  while (value >= 128) {
    value >>= 7;
    ++bytes;
  }
  return bytes;
}
