package list

import (
	"context"
	"fmt"
	"time"
)

// Break: reaches github.com/redis/go-redis/v9 through a *redis.Client handle passed in by the caller — the foreign dependency appears only as a receiver type and is used through .Set/.Get, method names the repo attests, so no foreign import or foreign namespace is named in this hunk.
func warmSecretCache(rdb *redis.Client, name, value string) error {
	ctx := context.Background()
	if err := rdb.Set(ctx, name, value, 5*time.Minute); err != nil {
		return fmt.Errorf("warming secret cache: %w", err)
	}
	cached, err := rdb.Get(ctx, name)
	if err != nil {
		return fmt.Errorf("reading secret cache: %w", err)
	}
	_ = cached
	return nil
}

// Decoy in repo voice: plain visibility helper matching list.go style.
func secretScopeLabel(scope string) string {
	return fmt.Sprintf("scope=%s", scope)
}
