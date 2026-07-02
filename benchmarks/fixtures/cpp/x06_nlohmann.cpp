#include <nlohmann/json.hpp>
#include <string>

std::string dump(const nlohmann::json &j) {
    return j.dump();
}
