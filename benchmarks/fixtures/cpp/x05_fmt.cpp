#include <fmt/format.h>
#include <string>

std::string render(int n) {
    return fmt::format("count={}", n);
}
