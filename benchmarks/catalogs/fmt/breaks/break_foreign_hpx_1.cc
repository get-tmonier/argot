// Break fixture -- not for compilation into the build.
#include "fmt/format.h"
#include <string>
#include <vector>

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- format one value.
inline auto format_one(int value) -> std::string {
  return fmt::format("{}", value);
}

// Break: HPX distributed async runtime -- work is launched with hpx::async and
// Break: joined through the returned future receiver. Zero `hpx::async` /
// Break: `hpx::future` sites in the corpus at the pinned SHA; {fmt} runs every
// Break: format inline on the calling thread and never offloads onto an HPX
// Break: executor (a foreign concurrency runtime, not a raw std::thread).
inline auto format_async(int value) -> std::string {
  hpx::future<std::string> f = hpx::async([value] {
    return fmt::format("{}", value);
  });
  return f.get();
}

}  // namespace fmt
