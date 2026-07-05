# frozen_string_literal: true

# Break fixture — not for require.
require 'time'

module RuboCop
  # Decoy helper in the RemoteConfig voice — NOT inside the hunk range.
  def self.cache_expired?(cache_path, lifetime)
    return true unless File.exist?(cache_path)

    (Time.now - File.stat(cache_path).mtime).to_f > lifetime
  end

  # Break: HTTParty to fetch an inherited remote config document. Verified
  # foreign at the pinned SHA: `httparty` is absent from rubocop.gemspec and
  # the Gemfile, and `HTTParty` = 0 grep hits across *.rb. The corpus fetches
  # remote configs with the stdlib Net::HTTP client in remote_config.rb.
  require 'httparty'

  def self.download_remote_config(url)
    response = HTTParty.get(url, headers: { 'Accept' => 'text/yaml' })
    raise ConfigNotFoundError, "config fetch failed: #{response.code}" unless response.code == 200

    response.body
  end
end
