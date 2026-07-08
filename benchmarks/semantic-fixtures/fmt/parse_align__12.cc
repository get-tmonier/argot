# ID: include/fmt/format.h:2339
// Parses an optional fill character and alignment specifier into specs.
template <typename Char>
auto scan_alignment(const Char* begin, const Char* end, format_specs& specs)
    -> const Char* {
  FMT_ASSERT(begin != end, "");
  auto alignment = align::none;
  auto p = begin + code_point_length(begin);
  if (end - p <= 0) p = begin;
  for (;;) {
    switch (to_ascii(*p)) {
    case '<': alignment = align::left; break;
    case '>': alignment = align::right; break;
    case '^': alignment = align::center; break;
    }
    if (alignment != align::none) {
      if (p == begin) {
        ++begin;
      } else {
        auto c = *begin;
        if (c == '}') return begin;
        if (c == '{') {
          report_error("invalid fill character '{'");
          return begin;
        }
        specs.set_fill(basic_string_view<Char>(begin, to_unsigned(p - begin)));
        begin = p + 1;
      }
      break;
    }
    if (p == begin) break;
    p = begin;
  }
  specs.set_align(alignment);
  return begin;
}
