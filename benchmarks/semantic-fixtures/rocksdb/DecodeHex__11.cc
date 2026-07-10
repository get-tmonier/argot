# ID: util/slice.cc:299
bool HexToBytes(const char* data, size_t size, std::string* result) {
  // Hex string must be an even number of digits to form whole bytes
  if (!result || (size % 2)) {
    return false;
  }
  result->clear();
  result->reserve(size / 2);
  for (size_t i = 0; i < size;) {
    int h1 = fromHex(data[i++]);
    if (h1 < 0) {
      return false;
    }
    int h2 = fromHex(data[i++]);
    if (h2 < 0) {
      return false;
    }
    result->push_back(static_cast<char>((h1 << 4) | h2));
  }
  return true;
}
