package hugolib
import "github.com/go-redis/redis/v8"
func cacheSet(c *redis.Client, k, v string) error { return c.Set(nil, k, v, 0).Err() }
