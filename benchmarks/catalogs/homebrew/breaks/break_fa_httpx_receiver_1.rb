# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(endpoint: String).returns(String) }
  def self.api_endpoint_url(endpoint)
    "https://formulae.brew.sh/api/#{endpoint}"
  end

  # Break: HTTPX reached through a session receiver variable —
  # `session = HTTPX.plugin(...)` then `session.get(...)` (medium: receiver
  # indirection). Verified at pinned SHA: httpx absent from Library/Homebrew/Gemfile
  # and 0 occurrences in own source; the distinctive callee `HTTPX.plugin` =
  # 0 src hits; the corpus fetches its API through Utils::Curl and DownloadQueue.
  require "httpx"

  sig { params(endpoint: String).returns(String) }
  def self.fetch_api_with_session(endpoint)
    session = HTTPX.plugin(:retries).with(timeout: { operation_timeout: 30 })
    response = session.get("https://formulae.brew.sh/api/#{endpoint}")
    response.body.to_s
  end
end
