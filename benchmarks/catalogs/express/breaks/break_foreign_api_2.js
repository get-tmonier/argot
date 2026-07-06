var Redis = require('ioredis');
var redis = new Redis();

// Break: res.cacheJson caches the rendered payload in Redis before writing
// it out. Express has no cache-store dependency; 'ioredis' is 0-usage in
// the repo at the pinned SHA.
res.cacheJson = function cacheJson(key, obj) {
  redis.setex(key, 60, JSON.stringify(obj));
  return this.json(obj);
};
