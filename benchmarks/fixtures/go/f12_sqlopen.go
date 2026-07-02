package cobra
import "database/sql"
func connect(dsn string) (*sql.DB, error) { db, err := sql.Open("postgres", dsn); return db, err }
