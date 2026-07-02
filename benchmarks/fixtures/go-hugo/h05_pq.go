package hugolib
import "database/sql"
import _ "github.com/lib/pq"
func openDB(dsn string) (*sql.DB, error) { return sql.Open("postgres", dsn) }
