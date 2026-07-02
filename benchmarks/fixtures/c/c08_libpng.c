#include <png.h>

png_structp make_writer(void) {
    return png_create_write_struct(PNG_LIBPNG_VER_STRING, NULL, NULL, NULL);
}
