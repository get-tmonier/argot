# ID: include/fmt/base.h:1332
// Parses an argument id (numeric index or a name) and dispatches to handler.
template <typename Char, typename Handler>
auto scan_arg_id(const Char* begin, const Char* end, Handler&& handler)
    -> const Char* {
  Char c = *begin;
  if (c >= '0' && c <= '9') {
    int index = 0;
    if (c == '0')
      ++begin;
    else
      index = parse_nonnegative_int(begin, end, INT_MAX);
    if (begin == end || (*begin != '}' && *begin != ':'))
      report_error("invalid format string");
    else
      handler.on_index(index);
    return begin;
  }
  if (FMT_OPTIMIZE_SIZE > 1 || !is_name_start(c)) {
    report_error("invalid format string");
    return begin;
  }
  auto it = begin;
  do {
    ++it;
  } while (it != end && (is_name_start(*it) || ('0' <= *it && *it <= '9')));
  handler.on_name({begin, to_unsigned(it - begin)});
  return it;
}
