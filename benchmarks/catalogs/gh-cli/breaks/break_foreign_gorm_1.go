package config

import (
	"fmt"
)

// Break: pulls in gorm.io/gorm to persist config entries to a SQL database.
import (
	"gorm.io/driver/sqlite"
	"gorm.io/gorm"
)

type configEntry struct {
	gorm.Model
	Host  string
	Key   string
	Value string
}

func openConfigDB(path string) (*gorm.DB, error) {
	db, err := gorm.Open(sqlite.Open(path), &gorm.Config{})
	if err != nil {
		return nil, fmt.Errorf("opening config db: %w", err)
	}
	if err := db.AutoMigrate(&configEntry{}); err != nil {
		return nil, fmt.Errorf("migrating config db: %w", err)
	}
	return db, nil
}

// Decoy in repo voice: plain host-key join matching config.go style.
func configCacheKey(host, key string) string {
	return fmt.Sprintf("%s:%s", host, key)
}
