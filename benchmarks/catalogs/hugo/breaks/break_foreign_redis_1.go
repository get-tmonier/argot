package filecache

import (
	"time"
)

// Decoy in repo voice: TTL check matching the filecache expiry style.
func isExpired(modTime time.Time, maxAge time.Duration) bool {
	if maxAge < 0 {
		return false
	}
	return time.Since(modTime) > maxAge
}

// Break: pulls in github.com/redis/go-redis/v9 to back the file cache with a
// remote Redis store. At the pinned SHA go-redis appears in zero .go files and
// is absent from go.mod; Hugo's caches persist to the local filesystem through
// github.com/spf13/afero, never a networked key-value store.
import (
	"github.com/redis/go-redis/v9"
)

func newRedisCacheBackend(addr string) *redis.Client {
	return redis.NewClient(&redis.Options{
		Addr: addr,
		DB:   0,
	})
}
