# frozen_string_literal: true

# Break fixture — not for require.
require 'time'

module RuboCop
  # Decoy helper in the RemoteConfig voice — NOT inside the hunk range.
  def self.cache_path_for(uri, cache_root)
    File.join(cache_root, uri.host, uri.path.sub(%r{\A/}, ''))
  end

  # Break: Excon HTTP client (Excon::Connection) to fetch a remote config —
  # qualified foreign constant, no require. Verified foreign at the pinned
  # SHA: `excon` is absent from rubocop.gemspec and the Gemfile, and
  # `Excon` = 0 grep hits across *.rb. The corpus fetches remote configs
  # with the stdlib Net::HTTP client in remote_config.rb.
  def self.fetch_remote(url)
    connection = Excon::Connection.new(url)
    response = connection.request(method: :get, path: '/.rubocop.yml')
    response.body
  end
end
