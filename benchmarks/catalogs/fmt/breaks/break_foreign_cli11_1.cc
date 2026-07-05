// Break fixture -- not for compilation into the build.
#include "fmt/format.h"
#include <string>

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- format a usage line.
inline auto usage_line(const std::string& prog) -> std::string {
  return fmt::format("usage: {} [options]", prog);
}

// Break: CLI11 command-line parser dependency for a demo entry point,
// Break: #include <CLI/CLI.hpp> inside the hunk. Zero `#include <CLI/` and zero
// Break: `CLI::App` sites in the corpus at the pinned SHA; {fmt} ships no
// Break: command-line front end and never links a third-party CLI parser.
#include <CLI/CLI.hpp>

inline int run_cli(int argc, char** argv) {
  CLI::App app{"fmt formatting demo"};
  std::string spec = "{}";
  app.add_option("-s,--spec", spec, "format spec");
  CLI11_PARSE(app, argc, argv);
  return spec.empty() ? 1 : 0;
}

}  // namespace fmt
