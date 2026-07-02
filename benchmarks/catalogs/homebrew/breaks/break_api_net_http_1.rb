# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "net/http"
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(url: String).returns(T::Boolean) }
  def self.homebrew_hosted_url?(url)
    url.start_with?("https://ghcr.io/v2/homebrew/", "https://formulae.brew.sh/")
  end

  # Break: raw Net::HTTP request plumbing for a download, where brew
  # standardizes on its curl machinery (Utils::Curl#curl_download,
  # utils/curl.rb:283). Net::HTTP has zero call sites in the corpus; it
  # appears only inside a docstring example in utils.rb.
  sig { params(url: String, max_redirects: Integer).returns(String) }
  def self.fetch_url_body(url, max_redirects: 5)
    uri = URI.parse(url)
    max_redirects.times do
      response = Net::HTTP.start(uri.host, uri.port, use_ssl: uri.scheme == "https") do |http|
        request = Net::HTTP::Get.new(uri)
        request["User-Agent"] = HOMEBREW_USER_AGENT_CURL
        http.request(request)
      end
      case response
      when Net::HTTPSuccess
        return response.body
      when Net::HTTPRedirection
        uri = URI.parse(response["location"])
      else
        raise CurlDownloadStrategyError, url
      end
    end
    raise CurlDownloadStrategyError, url
  end
end
