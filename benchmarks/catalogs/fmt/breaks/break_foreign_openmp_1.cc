// Break fixture -- not for compilation into the build.
#include "fmt/format.h"
#include <string>
#include <vector>
// Break: OpenMP runtime reached through BARE library calls -- the #include
// Break: <omp.h> sits here in the decoy region, OUTSIDE the scored hunk, so the
// Break: scored lines call omp_set_num_threads()/omp_get_thread_num() bare.
// Break: Zero `omp.h` / `omp_get_thread_num` sites in the corpus at the pinned
// Break: SHA; {fmt} formats synchronously and never dispatches onto the OpenMP
// Break: parallel runtime.
#include <omp.h>

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- format a batch sequentially.
inline auto format_batch(const std::vector<int>& values) -> std::vector<std::string> {
  std::vector<std::string> out;
  for (int v : values) out.push_back(fmt::format("{}", v));
  return out;
}

inline void format_batch_omp(const std::vector<int>& values,
                             std::vector<std::string>& out) {
  out.resize(values.size());
  omp_set_num_threads(4);
  for (int i = 0; i < static_cast<int>(values.size()); ++i) {
    int tid = omp_get_thread_num();
    out[i] = fmt::format("[t{}] {}", tid, values[i]);
  }
}

}  // namespace fmt
