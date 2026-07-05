# frozen_string_literal: true

# Break fixture — not for require.
require 'time'

module RuboCop
  # Decoy helper in the RemoteConfig voice — NOT inside the hunk range.
  def self.cache_name_from_uri(uri)
    File.basename(uri.path).gsub(/\.ya?ml\z/i, '')
  end

  # Break: Typhoeus parallel HTTP fetch of several inherited remote configs.
  # Verified foreign at the pinned SHA: `typhoeus` is absent from
  # rubocop.gemspec and the Gemfile, and `Typhoeus::Request`/`Typhoeus::` = 0
  # grep hits across *.rb. The corpus fetches configs one at a time through
  # the stdlib Net::HTTP client in remote_config.rb.
  def self.fetch_all_configs(urls)
    hydra = Typhoeus::Hydra.new(max_concurrency: 4)
    requests = urls.map do |url|
      request = Typhoeus::Request.new(url, followlocation: true)
      hydra.queue(request)
      request
    end
    hydra.run
    requests.map { |request| request.response.body }
  end
end
