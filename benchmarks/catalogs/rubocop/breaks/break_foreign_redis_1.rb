# frozen_string_literal: true

# Break fixture — not for require.
require 'digest/sha1'

module RuboCop
  # Decoy helper in the ResultCache voice — NOT inside the hunk range.
  def self.file_checksum(file, signature)
    digester = Digest::SHA1.new
    digester.update("#{file}#{signature}")
    digester.file(file)
    digester.hexdigest
  end

  # Break: Redis-backed offense cache. Verified foreign at the pinned SHA:
  # `redis` is absent from rubocop.gemspec and the Gemfile, and `Redis.new`
  # = 0 grep hits across *.rb. The corpus caches results on the local
  # filesystem under rubocop_cache, never a network cache server.
  require 'redis'

  def self.cache_offenses(path, offenses)
    client = Redis.new(url: ENV.fetch('RUBOCOP_REDIS_URL', 'redis://localhost:6379'))
    client.set("offenses:#{path}", offenses.to_json)
    client.expire("offenses:#{path}", 86_400)
  end
end
