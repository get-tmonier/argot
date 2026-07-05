// Break fixture -- not for compilation into the build.
#include "fmt/format.h"
#include <string>

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- render a spec label via fmt::format.
inline auto describe_json_spec(int width, int precision) -> std::string {
  return fmt::format("width={} precision={}", width, precision);
}

// Break: rapidjson Document/Writer for a JSON dump of a format spec. Zero
// Break: `#include <rapidjson` / `rapidjson::Document` sites in the corpus at
// Break: the pinned SHA; {fmt} assembles every string through its own
// Break: fmt::format / memory_buffer and links no third-party JSON library.
#include <rapidjson/document.h>
#include <rapidjson/writer.h>
#include <rapidjson/stringbuffer.h>

inline auto dump_spec_json(int width, int precision) -> std::string {
  rapidjson::Document doc;
  doc.SetObject();
  auto& alloc = doc.GetAllocator();
  doc.AddMember("width", width, alloc);
  doc.AddMember("precision", precision, alloc);
  rapidjson::StringBuffer buffer;
  rapidjson::Writer<rapidjson::StringBuffer> writer(buffer);
  doc.Accept(writer);
  return buffer.GetString();
}

}  // namespace fmt
