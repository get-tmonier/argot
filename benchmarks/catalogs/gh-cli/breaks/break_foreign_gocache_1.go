package list

import (
	"time"
)

// Break: reaches github.com/patrickmn/go-cache through a *cache.Cache handle passed in by the caller — the foreign dependency is only a receiver type and is used through .Set/.Get, method names the repo attests, so no foreign import or namespace is named in this hunk.
func cacheGistPage(c *cache.Cache, key string, page []byte) []byte {
	c.Set(key, page, 30*time.Second)
	if cached, ok := c.Get(key); ok {
		if b, ok := cached.([]byte); ok {
			return b
		}
	}
	return page
}

// Decoy in repo voice: plain filter helper matching list.go style.
func gistVisibilityLabel(public bool) string {
	if public {
		return "public"
	}
	return "secret"
}
