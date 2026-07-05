# frozen_string_literal: true

# Break fixture — not for require.
require 'time'

module RuboCop
  # Decoy helper in the RemoteConfig voice — NOT inside the hunk range.
  def self.cloned_url(uri)
    cloned = uri.clone
    cloned.user = nil if cloned.user
    cloned
  end

  # Break: Faraday HTTP client (with retry middleware) to fetch a remote
  # config. Verified foreign at the pinned SHA: `faraday` is absent from
  # rubocop.gemspec and the Gemfile, and `Faraday::Connection`/`Faraday::`
  # = 0 grep hits across *.rb. The corpus uses the stdlib Net::HTTP client.
  def self.fetch_config_body(url)
    connection = Faraday::Connection.new(url: url) do |builder|
      builder.request(:retry, max: 2)
      builder.response(:raise_error)
    end
    connection.get('/.rubocop.yml').body
  rescue Faraday::ConnectionFailed => e
    raise ConfigNotFoundError, "remote config unreachable: #{e.message}"
  end
end
