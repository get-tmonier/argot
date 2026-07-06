// Break fixture -- not for compilation into the build.
#include "fmt/format.h"
#include <string>

namespace fmt {

// Decoy: idiomatic {fmt}-style helper -- format a timestamp as text.
inline auto stamp_label(long long secs) -> std::string {
  return fmt::format("t={}", secs);
}

// Break: ICU (icu::SimpleDateFormat) drives the date formatting through a
// Break: RECEIVER whose leaf method is `.format()` -- which collides exactly
// Break: with {fmt}'s own attested `format` method, so the callee reads as
// Break: in-voice and the only foreign tokens are the icu:: TYPE names in the
// Break: declarations (no #include, no foreign namespace CALL). HARD: the
// Break: foreignness is masked behind an attested method leaf. Zero `icu::` /
// Break: `SimpleDateFormat` / `UnicodeString` sites in the corpus at the pinned
// Break: SHA; {fmt} formats dates through its own fmt/chrono.h, never ICU.
inline auto format_date_icu(double when, const std::string& pattern) -> std::string {
  UErrorCode status = U_ZERO_ERROR;
  icu::UnicodeString skeleton = pattern.c_str();
  icu::SimpleDateFormat sdf{skeleton, status};
  icu::UnicodeString out;
  sdf.format(when, out);
  std::string target;
  out.toUTF8String(target);
  return target;
}

}  // namespace fmt
