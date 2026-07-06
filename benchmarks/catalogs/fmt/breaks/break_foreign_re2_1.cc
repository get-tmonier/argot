// Break fixture -- not for compilation into the build.
#include "fmt/format.h"
#include <string>

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- echo a validated spec.
inline auto echo_spec(const std::string& spec) -> std::string {
  return fmt::format("spec: {}", spec);
}

// Break: RE2 regular-expression API to validate a replacement-field spec,
// Break: #include <re2/re2.h> inside the hunk. Zero `#include <re2/` and zero
// Break: `RE2::FullMatch` sites in the corpus at the pinned SHA; {fmt} scans
// Break: replacement fields with its own detail::parse_format_specs char loop
// Break: and never links a regular-expression engine.
#include <re2/re2.h>

inline bool is_valid_field(const std::string& field) {
  int width = 0;
  return RE2::FullMatch(field, R"(\{:(\d+)\})", &width) && width >= 0;
}

}  // namespace fmt
