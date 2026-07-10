# ID: include/fmt/format-inl.h:92
// Formats "<message>: error <code>" into out, staying within inline_buffer_size.
void build_error_message(detail::buffer<char>& out, int error_code,
                         string_view message) noexcept {
  out.try_resize(0);
  static constexpr char SEP[] = ": ";
  static constexpr char ERROR_STR[] = "error ";
  // Subtract 2 for the terminating nulls of SEP and ERROR_STR.
  size_t error_code_size = sizeof(SEP) + sizeof(ERROR_STR) - 2;
  auto abs_value = static_cast<uint32_or_64_or_128_t<int>>(error_code);
  if (detail::is_negative(error_code)) {
    abs_value = 0 - abs_value;
    ++error_code_size;
  }
  error_code_size += detail::to_unsigned(detail::count_digits(abs_value));
  auto it = appender(out);
  if (message.size() <= inline_buffer_size - error_code_size)
    fmt::format_to(it, FMT_STRING("{}{}"), message, SEP);
  fmt::format_to(it, FMT_STRING("{}{}"), ERROR_STR, error_code);
  FMT_ASSERT(out.size() <= inline_buffer_size, "");
}
