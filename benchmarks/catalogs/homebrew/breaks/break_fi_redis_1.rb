# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(endpoint: String).returns(String) }
  def self.api_cache_key(endpoint)
    endpoint.tr("/", "_")
  end

  # Break: require "redis" + Redis.new to cache API responses in a network
  # cache server. Verified at pinned SHA: redis absent from Library/Homebrew/Gemfile
  # and 0 word-boundary occurrences in own source (the lone substring hit is
  # "VCRedist" in bundle/extensions/winget.rb); the corpus caches API JSON on
  # the local filesystem via Homebrew::API, never a network cache server.
  require "redis"

  sig { params(endpoint: String, payload: String).void }
  def self.cache_api_response(endpoint, payload)
    redis = Redis.new(url: ENV.fetch("HOMEBREW_REDIS_URL", "redis://localhost:6379"))
    redis.set("api:#{endpoint}", payload, ex: 3600)
    redis.close
  end
end
