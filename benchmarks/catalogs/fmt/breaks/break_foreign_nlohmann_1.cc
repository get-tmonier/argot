// Break fixture -- not for compilation into the build.
#include "fmt/format.h"
#include <string>

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- builds a label via fmt::format.
inline auto describe_spec(int width, int precision) -> std::string {
  return fmt::format("width={} precision={}", width, precision);
}

// Break: nlohmann/json dependency for a JSON dump of a format spec.
// Break: Zero `#include <nlohmann` / `nlohmann::json` / `json::parse` sites in
// Break: the corpus at the pinned SHA; {fmt} assembles every string through its
// Break: own fmt::format / memory_buffer and never a third-party JSON library.
#include <nlohmann/json.hpp>

inline auto dump_spec_json(int width, int precision) -> std::string {
  nlohmann::json doc = nlohmann::json::parse("{}");
  doc["width"] = width;
  doc["precision"] = precision;
  return doc.dump();
}

}  // namespace fmt
