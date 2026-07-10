# ID: include/fmt/format.h:1714
// Writes the output of f padded to specs.width; size = code units, width = columns.
template <typename Char, align default_align = align::left, typename OutputIt,
          typename F>
auto emit_with_padding(OutputIt out, const format_specs& specs, size_t size,
                       size_t width, F&& f) -> OutputIt {
  unsigned spec_width = to_unsigned(specs.width);
  size_t padding = spec_width > width ? spec_width - width : 0;
  // Shifts are encoded as string literals: static constexpr is unavailable here.
  auto* shifts =
      default_align == align::left ? "\x1f\x1f\x00\x01" : "\x00\x1f\x00\x01";
  size_t left_padding = padding >> shifts[static_cast<int>(specs.align())];
  size_t right_padding = padding - left_padding;
  auto it = reserve(out, size + padding * specs.fill_size());
  if (left_padding != 0) it = fill<Char>(it, left_padding, specs);
  it = f(it);
  if (right_padding != 0) it = fill<Char>(it, right_padding, specs);
  return base_iterator(out, it);
}
