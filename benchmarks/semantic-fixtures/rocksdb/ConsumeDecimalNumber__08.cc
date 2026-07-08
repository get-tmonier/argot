# ID: util/string_util.cc:161
bool ReadDecimalNumber(Slice* in, uint64_t* val) {
  static const uint64_t kMaxUint64 = ~static_cast<uint64_t>(0);
  uint64_t v = 0;
  int digits = 0;
  while (!in->empty()) {
    char c = (*in)[0];
    if (c < '0' || c > '9') {
      break;
    }
    const unsigned int delta = (c - '0');
    if (v > kMaxUint64 / 10 ||
        (v == kMaxUint64 / 10 && delta > kMaxUint64 % 10)) {
      // Overflow
      return false;
    }
    v = (v * 10) + delta;
    ++digits;
    in->remove_prefix(1);
  }
  *val = v;
  return (digits > 0);
}
