package cmd

import (
	"gorm.io/driver/postgres"
	"gorm.io/gorm"
)

func openDB(dsn string) (*gorm.DB, error) {
	return gorm.Open(postgres.Open(dsn), &gorm.Config{})
}
