// Break fixture -- not for compilation into the build.
#include "fmt/format.h"
#include <string>

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- report via the repo's own channel.
inline void note_buffer_size(size_t size) {
  auto msg = fmt::format("buffer grew to {} bytes", size);
  (void)msg;
}

// Break: Google glog logging dependency for an oversized-buffer warning,
// Break: #include <glog/logging.h> inside the hunk. Zero `#include <glog/` and
// Break: zero `google::InitGoogleLogging` / `LOG(` sites in the corpus at the
// Break: pinned SHA; {fmt} is a formatting library with no logging facility and
// Break: reports errors via its own report_system_error helper, never glog.
#include <glog/logging.h>

inline void warn_oversized_buffer(const char* argv0, size_t size) {
  google::InitGoogleLogging(argv0);
  if (size > (1u << 20)) {
    LOG(WARNING) << "format buffer exceeded 1 MiB: " << size;
  }
  google::FlushLogFiles(google::GLOG_INFO);
}

}  // namespace fmt
