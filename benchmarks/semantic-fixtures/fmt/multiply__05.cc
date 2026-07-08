# ID: include/fmt/format.h:1659
// Computes lhs * rhs / pow(2, 64) rounded to nearest, half-up tie breaking.
auto high_word_product(uint64_t lhs, uint64_t rhs) -> uint64_t {
  // Split each operand into 32-bit halves and cross-multiply.
  uint64_t mask = (1ULL << 32) - 1;
  uint64_t a = lhs >> 32, b = lhs & mask;
  uint64_t c = rhs >> 32, d = rhs & mask;
  uint64_t bd = b * d, bc = b * c, ad = a * d, ac = a * c;
  // Assemble the middle 64 bits, adding a half for round-to-nearest.
  uint64_t mid = (bd >> 32) + (ad & mask) + (bc & mask) + (1U << 31);
  return ac + (ad >> 32) + (bc >> 32) + (mid >> 32);
}
