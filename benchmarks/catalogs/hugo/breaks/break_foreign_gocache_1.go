package dynacache

import (
	"time"
)

// Decoy in repo voice: TTL guard matching the dynacache package.
func clampTTL(ttl, max time.Duration) time.Duration {
	if ttl <= 0 || ttl > max {
		return max
	}
	return ttl
}

// Break: reaches into github.com/patrickmn/go-cache (aliased gocache) through
// a receiver variable — gocache.New(...) binds a foreign *Cache, then every
// use is store.Set(...) / store.Get(...). The constructor's leaf method (New)
// collides with the repo's own pervasive New(), and Set/Get go through the
// local receiver store, so no callee names a foreign namespace: a genuinely
// masked foreign API that may not fire. At the pinned SHA patrickmn/go-cache
// appears in zero .go files and is absent from go.mod; dynacache holds values
// in memory through github.com/bep/lazycache, never a foreign TTL cache.
func cachePartitionValue(key string, val any) (any, bool) {
	store := gocache.New(5*time.Minute, 10*time.Minute)
	store.Set(key, val, gocache.DefaultExpiration)
	return store.Get(key)
}
