#include <sqlite3.h>

int open_db(const char *path) {
    sqlite3 *db;
    return sqlite3_open(path, &db);
}
