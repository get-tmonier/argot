// Break fixture -- not for compilation into the build.
#include "fmt/format.h"
#include <string>
// Break: toml++ config dependency reached as a BARE callee -- the #include and
// Break: `using namespace toml;` sit here in the decoy region, OUTSIDE the
// Break: scored hunk, so the scored lines call parse_file() unqualified. Zero
// Break: `toml++` / `toml::parse_file` sites in the corpus at the pinned SHA;
// Break: {fmt} reads no config files and parses text only through its own
// Break: detail parsers, never a third-party TOML library.
#include <toml++/toml.h>
using namespace toml;

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- default width label.
inline auto default_width_label() -> std::string {
  return fmt::format("width={}", 0);
}

inline auto load_format_width(const std::string& path) -> int {
  auto config = parse_file(path);
  auto node = config["format"]["width"];
  return node.value_or(0);
}

}  // namespace fmt
