// Break fixture -- not for compilation into the build.
#include "fmt/format.h"
#include <string>

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- render a spec label.
inline auto spec_label(int width, int precision) -> std::string {
  return fmt::format("width={} precision={}", width, precision);
}

// Break: jsoncpp (Json::Reader / Json::Value) parses a spec through a RECEIVER
// Break: whose leaf methods are `.parse()` and `.get()` -- both collide exactly
// Break: with {fmt}'s own attested `parse`/`get` methods, so the callees read
// Break: as in-voice; the only foreign tokens are the Json:: TYPE names in the
// Break: declarations (no #include, no foreign namespace CALL). HARD: the
// Break: foreignness is masked behind attested method leaves. Zero `Json::` /
// Break: `Json::Reader` / `json/json.h` sites in the corpus at the pinned SHA;
// Break: {fmt} builds strings with fmt::format and links no JSON library.
inline auto parse_spec_json(const std::string& payload) -> int {
  Json::Reader reader;
  Json::Value root;
  if (!reader.parse(payload, root)) return 0;
  return root.get("width", 0).asInt();
}

}  // namespace fmt
