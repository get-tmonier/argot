#include <cpprest/http_client.h>

web::http::client::http_client make_client(const utility::string_t &u) {
    return web::http::client::http_client(u);
}
