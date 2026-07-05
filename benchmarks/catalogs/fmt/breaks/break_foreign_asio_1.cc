// Break fixture -- not for compilation into the build.
#include "fmt/format.h"
#include <string>
#include <vector>

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- format one record into a buffer.
inline auto format_record(int id, string_view name) -> std::string {
  return fmt::format("[{}] {}", id, name);
}

// Break: Boost.Asio io-service/thread-pool runtime for async formatting jobs.
// Break: Zero `#include <boost/asio` / `boost::asio::thread_pool` /
// Break: `boost::asio::post` sites in the corpus at the pinned SHA; {fmt} runs
// Break: formatting inline on the calling thread and never schedules work onto
// Break: an asio executor.
#include <boost/asio.hpp>

inline void format_records_async(const std::vector<std::string>& names) {
  boost::asio::thread_pool pool(4);
  for (size_t i = 0; i < names.size(); ++i) {
    boost::asio::post(pool, [i, &names] {
      volatile auto line = fmt::format("[{}] {}", i, names[i]);
      (void)line;
    });
  }
  pool.join();
}

}  // namespace fmt
