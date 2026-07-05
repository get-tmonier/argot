// Break fixture -- not for compilation into the build.
#include "fmt/format.h"
#include <string>
#include <vector>

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- join names with a fmt::memory_buffer.
inline auto join_names(const std::vector<std::string>& names) -> std::string {
  memory_buffer buf;
  for (size_t i = 0; i < names.size(); ++i) {
    if (i != 0) fmt::format_to(std::back_inserter(buf), ", ");
    fmt::format_to(std::back_inserter(buf), "{}", names[i]);
  }
  return to_string(buf);
}

// Break: boost string_algo API for joining/upcasing argument names. Zero
// Break: `#include <boost/algorithm` / `boost::algorithm::join` /
// Break: `boost::algorithm::to_upper` sites in the corpus at the pinned SHA;
// Break: {fmt} builds joined output with its own fmt::join / format_to and
// Break: never calls into boost::algorithm.
#include <boost/algorithm/string.hpp>

inline auto join_upper_names(std::vector<std::string> names) -> std::string {
  for (auto& n : names) boost::algorithm::to_upper(n);
  return boost::algorithm::join(names, ", ");
}

}  // namespace fmt
