# ID: include/fmt/base.h:1296
// Parses [begin, end) as an unsigned integer; assumes a leading digit.
template <typename Char>
auto read_unsigned_int(const Char*& begin, const Char* end, int error_value)
    -> int {
  FMT_ASSERT(begin != end && '0' <= *begin && *begin <= '9', "");
  unsigned value = 0, prev = 0;
  auto p = begin;
  do {
    prev = value;
    value = value * 10 + unsigned(*p - '0');
    ++p;
  } while (p != end && '0' <= *p && *p <= '9');
  auto num_digits = p - begin;
  begin = p;
  int digits10 = int(sizeof(int) * CHAR_BIT * 3 / 10);
  if (num_digits <= digits10) return int(value);
  // Guard against overflow of the last, potentially-too-big digit.
  unsigned max = INT_MAX;
  if (num_digits == digits10 + 1 && prev * 10ull + unsigned(p[-1] - '0') <= max)
    return int(value);
  return error_value;
}
