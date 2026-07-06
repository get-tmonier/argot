// Break fixture -- not for compilation into the build.
#include "fmt/format.h"
#include <string>
#include <vector>

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- describe a packed spec.
inline auto describe_packed(size_t bytes) -> std::string {
  return fmt::format("packed {} bytes", bytes);
}

// Break: MessagePack serializer reached through an ALIASED namespace
// Break: (`namespace mp = msgpack;`), then mp::pack() serializes a spec. Zero
// Break: `msgpack` / `mp::pack` sites in the corpus at the pinned SHA; {fmt}
// Break: has no binary-serialization surface -- it turns values into text with
// Break: fmt::format / memory_buffer and never packs to a wire format.
namespace mp = msgpack;

inline auto pack_spec(int width, int precision) -> std::string {
  mp::sbuffer sbuf;
  mp::pack(sbuf, width);
  mp::pack(sbuf, precision);
  return std::string(sbuf.data(), sbuf.size());
}

}  // namespace fmt
