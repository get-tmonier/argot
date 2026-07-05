# frozen_string_literal: true

# Break fixture — not for require.
require 'digest/sha1'

module RuboCop
  # Decoy helper in the ResultCache voice — NOT inside the hunk range.
  def self.cache_key(path, signature)
    Digest::SHA1.hexdigest("#{path}#{signature}")
  end

  # Break: Dalli (memcached) client caching offenses on a shared memcached
  # server. Verified foreign at the pinned SHA: `dalli` is absent from
  # rubocop.gemspec and the Gemfile, and `Dalli` = 0 grep hits across *.rb.
  # The corpus caches results on the local filesystem under rubocop_cache
  # (result_cache.rb / cache_config.rb), never a network cache server.
  require 'dalli'

  def self.cache_offenses(path, offenses)
    client = Dalli::Client.new(ENV.fetch('RUBOCOP_MEMCACHE', 'localhost:11211'))
    client.set("offenses:#{path}", offenses.to_json, 86_400)
  end
end
