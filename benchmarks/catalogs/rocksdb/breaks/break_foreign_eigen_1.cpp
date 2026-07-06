// Break fixture -- not for compilation into the build.
#include "db/table_cache.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- plain running average.
double MeanEntrySize(const std::vector<uint64_t>& sizes) {
  if (sizes.empty()) {
    return 0.0;
  }
  uint64_t total = 0;
  for (uint64_t s : sizes) {
    total += s;
  }
  return static_cast<double>(total) / static_cast<double>(sizes.size());
}

// Break: Eigen linear-algebra dependency to compute cache-cost statistics,
// Break: reached through fully-qualified Eigen:: constructors (no #include in
// Break: this hunk -- the callee itself is the foreign reference). Zero
// Break: `Eigen::` sites in the corpus at the pinned SHA (git grep); rocksdb
// Break: computes stats with plain scalar loops, never a matrix library.
double CacheCostStdDev(const std::vector<uint64_t>& sizes) {
  Eigen::VectorXd v = Eigen::VectorXd::Zero(static_cast<int>(sizes.size()));
  for (size_t i = 0; i < sizes.size(); ++i) {
    v[static_cast<int>(i)] = static_cast<double>(sizes[i]);
  }
  double mean = v.mean();
  Eigen::VectorXd centered = v.array() - mean;
  return std::sqrt(centered.dot(centered) / static_cast<double>(sizes.size()));
}

}  // namespace ROCKSDB_NAMESPACE
