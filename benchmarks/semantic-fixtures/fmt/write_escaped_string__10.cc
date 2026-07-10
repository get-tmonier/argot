# ID: include/fmt/format.h:1852
// Writes str wrapped in double quotes, escaping code points as needed.
template <typename Char, typename OutputIt>
auto emit_quoted_string(OutputIt out, basic_string_view<Char> str) -> OutputIt {
  *out++ = static_cast<Char>('"');
  auto begin = str.begin(), end = str.end();
  while (begin != end) {
    auto escape = find_escape(begin, end);
    out = copy<Char>(begin, escape.begin, out);
    begin = escape.end;
    if (!begin) break;
    out = write_escaped_cp<OutputIt, Char>(out, escape);
  }
  *out++ = static_cast<Char>('"');
  return out;
}
