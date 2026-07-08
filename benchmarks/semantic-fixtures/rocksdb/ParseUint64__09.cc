# ID: util/string_util.cc:323
uint64_t ConvertToUint64(const std::string& value) {
  size_t endchar;
#ifndef CYGWIN
  uint64_t num = std::stoull(value.c_str(), &endchar);
#else
  char* endptr;
  uint64_t num = std::strtoul(value.c_str(), &endptr, 0);
  endchar = endptr - value.c_str();
#endif

  if (endchar < value.length()) {
    char c = value[endchar];
    if (c == 'k' || c == 'K') {
      num <<= 10LL;
    } else if (c == 'm' || c == 'M') {
      num <<= 20LL;
    } else if (c == 'g' || c == 'G') {
      num <<= 30LL;
    } else if (c == 't' || c == 'T') {
      num <<= 40LL;
    }
  }
  return num;
}
