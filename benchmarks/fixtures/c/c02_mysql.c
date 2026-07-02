#include <mysql.h>

int connect_db(void) {
    MYSQL *conn = mysql_init(NULL);
    return conn ? mysql_real_connect(conn, "localhost", "u", "p", "d", 0, NULL, 0) != NULL : 0;
}
