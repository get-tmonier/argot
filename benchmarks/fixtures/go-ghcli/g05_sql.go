package cmd

import "database/sql"

func countUsers(db *sql.DB) (int, error) {
	var n int
	err := db.QueryRow("SELECT count(*) FROM users").Scan(&n)
	return n, err
}
