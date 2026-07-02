#include <json-c/json.h>

struct json_object *parse(const char *s) {
    return json_tokener_parse(s);
}
