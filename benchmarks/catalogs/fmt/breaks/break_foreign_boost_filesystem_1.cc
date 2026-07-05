// Break fixture -- not for compilation into the build.
#include "fmt/os.h"
#include <string>

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- open a file through fmt's own wrapper.
inline auto open_template(cstring_view path) -> buffered_file {
  return buffered_file(path, "r");
}

// Break: boost::filesystem dependency for template-file existence/size checks.
// Break: Zero `#include <boost/filesystem` / `boost::filesystem::exists` /
// Break: `boost::filesystem::file_size` sites in the corpus at the pinned SHA;
// Break: {fmt} touches the filesystem only through its own file/os.h wrappers
// Break: over POSIX/Win32 handles, never boost::filesystem.
#include <boost/filesystem.hpp>

inline bool template_file_is_usable(const std::string& path, size_t min_size) {
  boost::filesystem::path p(path);
  if (!boost::filesystem::exists(p)) return false;
  if (boost::filesystem::file_size(p) < min_size) return false;
  return boost::filesystem::is_regular_file(p);
}

}  // namespace fmt
