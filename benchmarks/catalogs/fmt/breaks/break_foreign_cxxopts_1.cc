// Break fixture -- not for compilation into the build.
#include "fmt/format.h"
#include <string>

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- summarise parsed options.
inline auto summarise(const std::string& spec, int count) -> std::string {
  return fmt::format("spec={} count={}", spec, count);
}

// Break: cxxopts option-parser reached through a receiver variable -- the
// Break: foreign type is built by a cxxopts::Options() call, then options are
// Break: added and parsed through the `options` receiver. Zero `cxxopts` sites
// Break: in the corpus at the pinned SHA; {fmt} has no command-line surface and
// Break: never links a third-party option parser.
inline int parse_demo_flags(int argc, char** argv) {
  auto options = cxxopts::Options("fmt-demo", "formatting demo");
  options.add_options()("s,spec", "format spec", cxxopts::value<std::string>());
  auto result = options.parse(argc, argv);
  return static_cast<int>(result.count("spec"));
}

}  // namespace fmt
