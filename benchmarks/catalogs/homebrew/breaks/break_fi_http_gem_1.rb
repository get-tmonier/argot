# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(url: String).returns(T::Boolean) }
  def self.downloadable_url?(url)
    url.start_with?("https://", "http://")
  end

  # Break: (HARD) the `http` gem's HTTP.get(url) client — the tell is masked
  # because the root token `HTTP` is heavily attested in the corpus (51
  # own-source hits, via Net::HTTP / HTTP status constants), and require "http"
  # resembles the stdlib "net/http" the corpus actually uses. Verified at
  # pinned SHA: the http gem absent from Library/Homebrew/Gemfile, `HTTP.get`
  # = 0 own-source hits; brew downloads through Utils::Curl#curl_download.
  require "http"

  sig { params(url: String).returns(String) }
  def self.fetch_via_http_gem(url)
    response = HTTP.get(url)
    raise CurlDownloadStrategyError, url unless response.status.success?

    response.body.to_s
  end
end
