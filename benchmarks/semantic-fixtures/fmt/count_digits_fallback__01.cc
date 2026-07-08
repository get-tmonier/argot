# ID: include/fmt/format.h:1053
// Returns the number of decimal digits of n without any lookup tables.
template <typename T> auto decimal_length(T n) -> int {
  int digits = 1;
  // Strip four digits at a time: integer division is slow, so amortise it
  // over groups of four (Alexandrescu's "Three Optimization Tips").
  while (n >= 10000) {
    n /= 10000u;
    digits += 4;
  }
  if (n < 10) return digits;
  if (n < 100) return digits + 1;
  if (n < 1000) return digits + 2;
  return digits + 3;
}
