package shared

import (
	"context"
	"fmt"
)

// Break: pulls in github.com/jmoiron/sqlx to cache gist metadata in a local
// SQL store. At the pinned SHA sqlx appears in zero files and is absent
// from go.mod; the repo has no SQL database anywhere — gists are read and
// written exclusively through api.Client REST/GraphQL calls.
import (
	"github.com/jmoiron/sqlx"
)

func cacheGistMetadata(ctx context.Context, db *sqlx.DB, gistID, description string) error {
	_, err := db.NamedExecContext(ctx, `
		INSERT INTO gist_cache (id, description, cached_at)
		VALUES (:id, :description, CURRENT_TIMESTAMP)
		ON CONFLICT(id) DO UPDATE SET description = :description
	`, map[string]interface{}{
		"id":          gistID,
		"description": description,
	})
	if err != nil {
		return fmt.Errorf("caching gist metadata: %w", err)
	}
	return nil
}

func openGistCache(path string) (*sqlx.DB, error) {
	return sqlx.Connect("sqlite3", path)
}

// Decoy in repo voice: plain struct mirroring the package's Gist shape.
type gistCacheEntry struct {
	ID          string
	Description string
}
