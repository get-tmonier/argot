#include <boost/algorithm/string.hpp>
#include <string>

std::string upper(std::string s) {
    boost::to_upper(s);
    return s;
}
