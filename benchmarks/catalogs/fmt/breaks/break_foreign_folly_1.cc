// Break fixture -- not for compilation into the build.
#include "fmt/format.h"
#include <string>
#include <vector>

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- format a batch on the caller's thread.
inline auto format_lines(const std::vector<int>& codes) -> std::vector<std::string> {
  std::vector<std::string> out;
  for (int code : codes) out.push_back(fmt::format("code {}", code));
  return out;
}

// Break: Facebook folly CPU thread-pool executor for parallel formatting.
// Break: Zero `#include <folly/` / `folly::CPUThreadPoolExecutor` sites in the
// Break: corpus at the pinned SHA; {fmt} owns no executor abstraction and
// Break: never offloads formatting onto a folly thread pool.
#include <folly/executors/CPUThreadPoolExecutor.h>

inline void format_lines_pooled(const std::vector<int>& codes) {
  folly::CPUThreadPoolExecutor executor(4);
  for (int code : codes) {
    executor.add([code] {
      volatile auto line = fmt::format("code {}", code);
      (void)line;
    });
  }
  executor.join();
}

}  // namespace fmt
