// Break fixture -- not for compilation into the build.
#include "db/compaction/compaction_job.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- sequential subcompaction bounds.
size_t CountSubcompactionBoundaries(const std::vector<Slice>& boundaries) {
  return boundaries.empty() ? 0 : boundaries.size() - 1;
}

// Break: Taskflow dependency, reached through an executor receiver variable to
// Break: schedule subcompaction jobs. Zero `#include <taskflow` and zero
// Break: `tf::Taskflow` sites in the corpus at the pinned SHA (git grep);
// Break: rocksdb runs subcompactions in a std::vector<port::Thread> and joins
// Break: every worker (compaction_job.cc:732,895), never a foreign scheduler.
#include <taskflow/taskflow.hpp>

void RunSubcompactionsWithTaskflow(const std::vector<int>& subcompactions) {
  tf::Executor executor(4);
  tf::Taskflow flow;
  for (int id : subcompactions) {
    flow.emplace([id]() { (void)id; });
  }
  executor.run(flow).wait();
}

}  // namespace ROCKSDB_NAMESPACE
