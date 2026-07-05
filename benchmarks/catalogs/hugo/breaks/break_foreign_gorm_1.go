package hugolib

import (
	"fmt"
)

// Decoy in repo voice: page cache-key builder matching pages_capture's style.
func pageCacheKey(section, name string) string {
	return fmt.Sprintf("%s/%s", section, name)
}

// Break: uses gorm.io/gorm to persist captured page metadata to a SQL
// database. At the pinned SHA gorm appears in zero .go files and is absent
// from go.mod; Hugo has no database layer — captured pages live in in-memory
// structures and the page bundle map, never an ORM.
import (
	"gorm.io/driver/sqlite"
	"gorm.io/gorm"
)

type capturedPage struct {
	gorm.Model
	Path    string
	Section string
}

func openPageStore(dsn string) (*gorm.DB, error) {
	db, err := gorm.Open(sqlite.Open(dsn), &gorm.Config{})
	if err != nil {
		return nil, fmt.Errorf("opening page store: %w", err)
	}
	if err := db.AutoMigrate(&capturedPage{}); err != nil {
		return nil, fmt.Errorf("migrating page store: %w", err)
	}
	return db, nil
}
