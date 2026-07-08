# ID: util/slice.cc:282
std::string SliceToString(const char* data, size_t size, bool hex) {
  std::string result;  // RVO/NRVO/move
  if (!hex) {
    result.assign(data, size);
    return result;
  }
  result.reserve(2 * size);
  for (size_t i = 0; i < size; ++i) {
    unsigned char c = data[i];
    result.push_back(toHex(c >> 4));
    result.push_back(toHex(c & 0xf));
  }
  return result;
}
