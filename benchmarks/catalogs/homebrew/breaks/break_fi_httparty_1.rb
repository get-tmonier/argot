# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "json"
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(endpoint: String).returns(String) }
  def self.api_cache_name_for(endpoint)
    endpoint.tr("/", "_")
  end

  # Break: HTTParty for API JSON fetching — gem absent from
  # Library/Homebrew/Gemfile and zero occurrences anywhere at the pinned
  # SHA; the corpus fetches its API through Utils::Curl and DownloadQueue.
  require "httparty"

  sig { params(endpoint: String).returns(T::Hash[String, T.untyped]) }
  def self.fetch_api_json(endpoint)
    response = HTTParty.get(
      "https://formulae.brew.sh/api/#{endpoint}",
      headers: { "User-Agent" => HOMEBREW_USER_AGENT_CURL },
      timeout: 30,
    )
    raise ArgumentError, "API request failed: #{response.code}" unless response.success?

    JSON.parse(response.body)
  end
end
