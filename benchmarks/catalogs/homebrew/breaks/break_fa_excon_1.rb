# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "tap"
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(name: String).returns(T::Boolean) }
  def self.tap_installed?(name)
    Tap.fetch(name).installed?
  end

  # Break: require "excon" + Excon.get for a tap manifest fetch. Verified at
  # pinned SHA: excon absent from Library/Homebrew/Gemfile and 0 occurrences
  # in own source; the distinctive callee `Excon.get` = 0 src hits; the corpus
  # fetches remote JSON through Utils::Curl and DownloadQueue, never Excon.
  require "excon"

  sig { params(url: String).returns(String) }
  def self.fetch_tap_manifest(url)
    response = Excon.get(url, headers: { "User-Agent" => HOMEBREW_USER_AGENT_CURL })
    raise ArgumentError, "manifest fetch failed: #{response.status}" unless response.status == 200

    response.body
  end
end
