# ID: include/fmt/format.h:1231
// Writes the decimal digits of value right-to-left into out, size wide.
template <typename Char, typename UInt>
auto emit_decimal_digits(Char* out, UInt value, int size) -> Char* {
  FMT_ASSERT(size >= count_digits(value), "invalid digit count");
  unsigned n = to_unsigned(size);
  // Peel two digits at a time to avoid per-digit integer division.
  while (value >= 100) {
    n -= 2;
    write2digits(out + n, static_cast<unsigned>(value % 100));
    value /= 100;
  }
  if (value < 10) {
    out[--n] = static_cast<Char>('0' + value);
    return out + n;
  }
  n -= 2;
  write2digits(out + n, static_cast<unsigned>(value));
  return out + n;
}
