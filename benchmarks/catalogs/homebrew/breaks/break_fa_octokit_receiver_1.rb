# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "tap"
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(name: String).returns(String) }
  def self.tap_remote(name)
    Tap.fetch(name).remote.to_s
  end

  # Break: Octokit GitHub client reached through a receiver variable —
  # `client = Octokit::Client.new(...)` then `client.contents(...)` (medium:
  # receiver indirection). Verified at pinned SHA: octokit absent from
  # Library/Homebrew/Gemfile and 0 occurrences in own source; the distinctive
  # callee `Octokit::Client` = 0 src hits; the corpus talks to GitHub through
  # its own GitHub::API + Utils::Curl, never the octokit gem.
  require "octokit"

  sig { params(repo: String, path: String).returns(String) }
  def self.fetch_tap_file(repo, path)
    client = Octokit::Client.new(access_token: ENV.fetch("HOMEBREW_GITHUB_API_TOKEN", nil))
    file = client.contents(repo, path: path)
    Base64.decode64(file.content)
  end
end
