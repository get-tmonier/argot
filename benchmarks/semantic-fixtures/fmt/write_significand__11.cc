# ID: include/fmt/format.h:2433
// Writes significand with a decimal point inserted before the fractional part.
template <typename Char, typename UInt>
auto emit_significand_with_point(Char* out, UInt significand, int significand_size,
                                 int integral_size, Char decimal_point) -> Char* {
  if (!decimal_point)
    return format_decimal(out, significand, significand_size);
  out += significand_size + 1;
  Char* end = out;
  int floating_size = significand_size - integral_size;
  // Emit the fractional digits two at a time, then any leftover single digit.
  for (int i = floating_size / 2; i > 0; --i) {
    out -= 2;
    write2digits(out, static_cast<size_t>(significand % 100));
    significand /= 100;
  }
  if (floating_size % 2 != 0) {
    *--out = static_cast<Char>('0' + significand % 10);
    significand /= 10;
  }
  *--out = decimal_point;
  format_decimal(out - integral_size, significand, integral_size);
  return end;
}
