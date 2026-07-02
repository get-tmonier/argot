#include <absl/strings/str_cat.h>
#include <string>

std::string join(const std::string &a, const std::string &b) {
    return absl::StrCat(a, b);
}
