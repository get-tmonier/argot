# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "tap"
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(name: String).returns(Integer) }
  def self.tap_formula_count(name)
    Tap.fetch(name).formula_files.count
  end

  # Break: Faraday HTTP client with retry middleware for tap manifest
  # fetches. Verified at pinned SHA: faraday absent from Gemfile and zero
  # occurrences anywhere; callees Faraday.new/`.status` = 0 src hits
  # (non-colliding); the corpus fetches JSON through Utils::Curl.
  require "faraday"

  sig { params(url: String).returns(String) }
  def self.fetch_remote_manifest(url)
    connection = Faraday.new(url: url) do |builder|
      builder.request :retry
      builder.adapter Faraday.default_adapter
    end
    response = connection.get("/manifest.json")
    raise ArgumentError, "manifest fetch failed: #{response.status}" unless response.status == 200

    response.body
  end
end
