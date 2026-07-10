# ID: include/fmt/format.h:1751
// Writes value as a "0x..." pointer, right-aligned when specs are given.
template <typename Char, typename OutputIt, typename UIntPtr>
auto emit_pointer(OutputIt out, UIntPtr value, const format_specs* specs)
    -> OutputIt {
  int num_digits = count_digits<4>(value);
  auto size = to_unsigned(num_digits) + size_t(2);
  auto write = [=](reserve_iterator<OutputIt> it) {
    *it++ = static_cast<Char>('0');
    *it++ = static_cast<Char>('x');
    return format_base2e<Char>(4, it, value, num_digits);
  };
  if (!specs) return base_iterator(out, write(reserve(out, size)));
  return write_padded<Char, align::right>(out, *specs, size, write);
}
