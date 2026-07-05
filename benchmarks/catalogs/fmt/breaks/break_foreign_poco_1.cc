// Break fixture -- not for compilation into the build.
#include "fmt/format.h"
#include <string>

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- render a parsed value back to text.
inline auto restate_number(double value, int precision) -> std::string {
  return fmt::format("{:.{}f}", value, precision);
}

// Break: POCO NumberParser API for parsing a numeric format argument. Zero
// Break: `#include <Poco/` / `Poco::NumberParser` / `Poco::NumberParser::
// Break: parseFloat` sites in the corpus at the pinned SHA; {fmt} converts
// Break: text through its own detail parsers (parse_float / to_unsigned) and
// Break: never links the POCO libraries.
#include <Poco/NumberParser.h>

inline auto parse_width_argument(const std::string& token) -> double {
  double value = 0.0;
  if (!Poco::NumberParser::tryParseFloat(token, value)) return 0.0;
  return Poco::NumberParser::parseFloat(token);
}

}  // namespace fmt
