// Break fixture -- not for compilation into the build.
#include "fmt/format.h"
#include <string>

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- format a diagnostic message.
inline auto truncation_message(size_t requested, size_t capacity) -> std::string {
  return fmt::format("output truncated: wanted {} bytes, buffer holds {}",
                     requested, capacity);
}

// Break: spdlog logging dependency for an oversized-buffer warning. Zero
// Break: `#include <spdlog/` / `spdlog::` sites in the corpus at the pinned
// Break: SHA; {fmt} is a formatting library with no logging facility of its
// Break: own and never reaches for a standalone logging framework.
#include <spdlog/spdlog.h>

inline void warn_on_truncation(size_t requested, size_t capacity) {
  if (requested > capacity)
    spdlog::warn("output truncated: wanted {} bytes, buffer holds {}",
                 requested, capacity);
}

}  // namespace fmt
