// Break fixture -- not for compilation into the build.
#include "fmt/format.h"
#include <string>
#include <vector>

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- pre-render labels into a vector.
inline auto render_labels(const std::vector<int>& codes) -> std::vector<std::string> {
  std::vector<std::string> out;
  for (int code : codes) out.push_back(fmt::format("arg-{}", code));
  return out;
}

// Break: abseil flat_hash_map + StrCat for an in-process label cache. Zero
// Break: `absl::flat_hash_map` / `absl::StrCat` sites in the corpus at the
// Break: pinned SHA (the only `absl::` identifiers live under the vendored,
// Break: muted test/gtest tree); {fmt} caches nothing in third-party
// Break: containers and builds strings with fmt::format, never absl.
#include <absl/container/flat_hash_map.h>
#include <absl/strings/str_cat.h>

inline auto build_label_cache(const std::vector<int>& codes)
    -> absl::flat_hash_map<int, std::string> {
  absl::flat_hash_map<int, std::string> cache;
  for (int code : codes) cache[code] = absl::StrCat("arg-", code);
  return cache;
}

}  // namespace fmt
