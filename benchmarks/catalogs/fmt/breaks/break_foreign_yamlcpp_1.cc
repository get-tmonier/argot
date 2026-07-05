// Break fixture -- not for compilation into the build.
#include "fmt/format.h"
#include <string>

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- default width label.
inline auto width_label(int width) -> std::string {
  return fmt::format("width={}", width);
}

// Break: yaml-cpp dependency to read format options from a YAML config,
// Break: #include <yaml-cpp/yaml.h> inside the hunk. Zero `#include <yaml-cpp/`
// Break: and zero `YAML::Load` sites in the corpus at the pinned SHA; {fmt}
// Break: reads no configuration files and parses text only through its own
// Break: detail parsers, never a third-party YAML library.
#include <yaml-cpp/yaml.h>

inline auto read_width_from_yaml(const std::string& text) -> int {
  YAML::Node config = YAML::Load(text);
  if (!config["format"]) return 0;
  return config["format"]["width"].as<int>();
}

}  // namespace fmt
