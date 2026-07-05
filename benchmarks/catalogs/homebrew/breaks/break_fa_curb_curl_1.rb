# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"
require "curb"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(url: String).returns(T::Boolean) }
  def self.curl_hosted_url?(url)
    url.start_with?("https://ghcr.io/")
  end

  # Break: (HARD) curb's Curl::Easy.new download — the tell is masked because
  # the root namespace `Curl` is one the corpus owns heavily as `Utils::Curl`
  # (56 own-source `Curl` hits). require "curb" sits in the decoy above; only
  # the `Curl::Easy` scope-resolution callee is in the scored hunk. Verified
  # at pinned SHA: curb absent from Library/Homebrew/Gemfile, `Curl::` = 0
  # own-source hits; brew downloads via Utils::Curl#curl_download, not curb.
  sig { params(url: String, path: Pathname).void }
  def self.download_with_curb(url, path)
    handle = Curl::Easy.new(url)
    handle.follow_location = true
    handle.on_success { |easy| path.write(easy.body_str) }
    handle.perform
  end
end
