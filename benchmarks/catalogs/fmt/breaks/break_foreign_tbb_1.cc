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

// Break: Intel TBB parallel task runtime for batch formatting. Zero
// Break: `#include <tbb/` / `tbb::parallel_for` / `tbb::blocked_range` sites
// Break: in the corpus at the pinned SHA; {fmt} is a synchronous formatting
// Break: library and never pulls in a parallel task runtime.
#include <tbb/parallel_for.h>
#include <tbb/blocked_range.h>

inline void format_batch_parallel(const std::vector<int>& values,
                                  std::vector<std::string>& out) {
  out.resize(values.size());
  tbb::parallel_for(tbb::blocked_range<size_t>(0, values.size()),
                    [&](const tbb::blocked_range<size_t>& range) {
                      for (size_t i = range.begin(); i != range.end(); ++i)
                        out[i] = fmt::format("{}", values[i]);
                    });
}

}  // namespace fmt
