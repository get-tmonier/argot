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

// Break: Taskflow parallel task-graph runtime for batch formatting,
// Break: #include <taskflow/taskflow.hpp> inside the hunk. Zero `#include
// Break: <taskflow/` and zero `tf::Executor` / `tf::Taskflow` sites in the
// Break: corpus at the pinned SHA; {fmt} is a synchronous formatting library
// Break: and never schedules work onto a task-graph executor.
#include <taskflow/taskflow.hpp>

inline void format_batch_parallel(const std::vector<int>& values,
                                  std::vector<std::string>& out) {
  out.resize(values.size());
  tf::Executor executor;
  tf::Taskflow taskflow;
  taskflow.for_each_index(size_t(0), values.size(), size_t(1), [&](size_t i) {
    out[i] = fmt::format("{}", values[i]);
  });
  executor.run(taskflow).wait();
}

}  // namespace fmt
