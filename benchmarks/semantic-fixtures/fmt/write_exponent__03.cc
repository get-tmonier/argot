# ID: include/fmt/format.h:1566
// Writes the exponent exp in the form "[+-]d{2,3}" to out.
template <typename Char, typename OutputIt>
auto emit_exponent(int exp, OutputIt out) -> OutputIt {
  FMT_ASSERT(-10000 < exp && exp < 10000, "exponent out of range");
  if (exp >= 0) {
    *out++ = static_cast<Char>('+');
  } else {
    *out++ = static_cast<Char>('-');
    exp = -exp;
  }
  auto uexp = static_cast<uint32_t>(exp);
  if (uexp >= 100u) {
    const char* top = digits2(uexp / 100);
    if (uexp >= 1000u) *out++ = static_cast<Char>(top[0]);
    *out++ = static_cast<Char>(top[1]);
    uexp %= 100;
  }
  const char* d = digits2(uexp);
  *out++ = static_cast<Char>(d[0]);
  *out++ = static_cast<Char>(d[1]);
  return out;
}
