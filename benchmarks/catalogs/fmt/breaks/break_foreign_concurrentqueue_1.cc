// Break fixture -- not for compilation into the build.
#include "fmt/format.h"
#include <string>
#include <vector>

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- format a batch sequentially.
inline auto format_batch(const std::vector<int>& values) -> std::vector<std::string> {
  std::vector<std::string> out;
  for (int v : values) out.push_back(fmt::format("{}", v));
  return out;
}

// Break: moodycamel::ConcurrentQueue (a foreign lock-free concurrency lib) is
// Break: used only through a local RECEIVER -- the queue is a plain declaration
// Break: (a foreign TYPE, not a namespace CALL) and the work goes through
// Break: `queue.enqueue()` / `queue.try_dequeue()` on that local variable, so
// Break: no foreign namespace ever appears in a call and there is no #include
// Break: in the fixture. HARD: the foreignness is masked as an ordinary local
// Break: object. Zero `moodycamel` / `ConcurrentQueue` sites in the corpus at
// Break: the pinned SHA; {fmt} formats synchronously with no queue of its own.
inline auto drain_formatted(const std::vector<int>& values) -> std::vector<std::string> {
  moodycamel::ConcurrentQueue<std::string> queue;
  for (int v : values) queue.enqueue(fmt::format("{}", v));
  std::vector<std::string> out;
  std::string item;
  while (queue.try_dequeue(item)) out.push_back(item);
  return out;
}

}  // namespace fmt
