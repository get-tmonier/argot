# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"
require "dalli"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  CACHE_CLIENT = T.let(Dalli::Client.new("localhost:11211"), T.untyped)

  # Decoy brew-style reader — NOT inside the hunk range.
  sig { returns(T.untyped) }
  def self.cache
    CACHE_CLIENT
  end

  # Break: (HARD) memcached (dalli) fetch-or-compute — the tell is masked to
  # the leaf method `.fetch`, which collides with the corpus's ubiquitous
  # `fetch` (836 own-source call sites, 17 `def fetch`). The distinctive
  # Dalli::Client constructor sits in the decoy above; only the colliding
  # `cache.fetch` receiver-call is in the scored hunk. Verified at pinned SHA:
  # dalli absent from Library/Homebrew/Gemfile, `Dalli` = 0 own-source hits;
  # brew memoizes metadata in-process / on disk, never a memcached client.
  sig { params(key: String).returns(String) }
  def self.cached_download_meta(key)
    cache.fetch(key) do
      "#{HOMEBREW_CACHE}/#{key}"
    end
  end
end
