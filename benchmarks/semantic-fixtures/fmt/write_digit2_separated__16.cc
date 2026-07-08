# ID: include/fmt/chrono.h:544
// Writes "aa<sep>bb<sep>cc" (8 chars) into buf using a branchless BCD trick.
void emit_two_digit_triplet(char* buf, unsigned a, unsigned b, unsigned c,
                            char sep) {
  ullong digits = a | (b << 24) | (static_cast<ullong>(c) << 48);
  // Convert each packed value to BCD: y = x + floor(x / 10) * 6.
  digits += (((digits * 205) >> 11) & 0x000f00000f00000f) * 6;
  // Swap high and low nibbles into byte positions.
  digits = ((digits & 0x00f00000f00000f0) >> 4) |
           ((digits & 0x000f00000f00000f) << 8);
  auto usep = static_cast<ullong>(sep);
  // Add ASCII '0' to every digit byte and drop the separators in.
  digits |= 0x3030003030003030 | (usep << 16) | (usep << 40);

  constexpr size_t len = 8;
  if (!is_big_endian()) {
    std::memcpy(buf, &digits, len);
    return;
  }
  char tmp[len];
  std::memcpy(tmp, &digits, len);
  std::reverse_copy(tmp, tmp + len, buf);
}
