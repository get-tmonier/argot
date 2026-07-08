# ID: include/fmt/format.h:1642
// Normalizes the value converted from double and multiplied by (1 << SHIFT).
template <int SHIFT = 0, typename F>
auto canonicalize_fp(basic_fp<F> value) -> basic_fp<F> {
  const auto implicit_bit = F(1) << num_significand_bits<double>();
  const auto shifted_implicit_bit = implicit_bit << SHIFT;
  // Shift the significand left until the hidden bit is set (handles subnormals).
  while ((value.f & shifted_implicit_bit) == 0) {
    value.f <<= 1;
    --value.e;
  }
  // Subtract 1 to account for the hidden bit.
  const auto offset = basic_fp<F>::num_significand_bits -
                      num_significand_bits<double>() - SHIFT - 1;
  value.e -= offset;
  value.f <<= offset;
  return value;
}
