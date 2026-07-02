require "redis"

def cache
  Redis.new(host: "localhost")
end
