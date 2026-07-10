// Break fixture -- not for compilation into the build.
#include "db/flush_job.h"
#include "rocksdb/status.h"

namespace ROCKSDB_NAMESPACE {

// Decoy: idiomatic rocksdb-style helper -- run flush callbacks in order.
void RunFlushCallbacks(const std::vector<std::function<void()>>& cbs) {
  for (const auto& cb : cbs) {
    cb();
  }
}

// Break: boost::asio::thread_pool, a foreign thread-pool runtime, to run flush
// Break: callbacks in parallel. Zero `boost::asio` and zero `#include
// Break: <boost/asio` sites in the corpus at the pinned SHA (git grep); rocksdb
// Break: schedules background work through its own Env thread pools, never a
// Break: foreign asio runtime.
#include <boost/asio/thread_pool.hpp>
#include <boost/asio/post.hpp>

void RunFlushCallbacksParallel(const std::vector<std::function<void()>>& cbs) {
  boost::asio::thread_pool pool(4);
  for (const auto& cb : cbs) {
    boost::asio::post(pool, cb);
  }
  pool.join();
}

}  // namespace ROCKSDB_NAMESPACE
