#include <expat.h>

XML_Parser make_parser(void) {
    return XML_ParserCreate(NULL);
}
