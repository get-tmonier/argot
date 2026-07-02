package cmd

import "github.com/go-redis/redis/v8"

func cacheGet(rdb *redis.Client, key string) (string, error) {
	return rdb.Get(ctxTODO(), key).Result()
}
