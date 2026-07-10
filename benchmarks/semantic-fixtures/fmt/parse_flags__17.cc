# ID: include/fmt/printf.h:314
// Consumes printf conversion flags (-+ 0#) from it, updating specs.
template <typename Char>
void scan_printf_flags(format_specs& specs, const Char*& it, const Char* end) {
  for (; it != end; ++it) {
    switch (*it) {
    case '-': specs.set_align(align::left); break;
    case '+': specs.set_sign(sign::plus); break;
    case '0': specs.set_fill('0'); break;
    case ' ':
      if (specs.sign() != sign::plus) specs.set_sign(sign::space);
      break;
    case '#': specs.set_alt(); break;
    default: return;
    }
  }
}
