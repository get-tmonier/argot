#include <Poco/URI.h>
#include <string>

std::string host(const std::string &u) {
    Poco::URI uri(u);
    return uri.getHost();
}
