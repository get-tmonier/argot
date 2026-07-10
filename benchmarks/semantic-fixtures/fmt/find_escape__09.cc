# ID: include/fmt/format.h:1780
// Scans [begin, end) for the first code point that needs escaping.
template <typename Char>
auto scan_for_escape(const Char* begin, const Char* end)
    -> find_escape_result<Char> {
  while (begin != end) {
    uint32_t cp = static_cast<unsigned_char<Char>>(*begin);
    // Multibyte lead/continuation bytes are handled by the UTF-8 overload.
    if (sizeof(Char) != 1 || cp < 0x80) {
      if (needs_escape(cp)) return {begin, begin + 1, cp};
    }
    ++begin;
  }
  return {begin, nullptr, 0};
}
