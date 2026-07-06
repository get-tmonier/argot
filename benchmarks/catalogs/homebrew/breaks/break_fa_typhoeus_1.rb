# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(urls: T::Array[String]).returns(Integer) }
  def self.queued_count(urls)
    urls.reject(&:empty?).count
  end

  # Break: require "typhoeus" + Typhoeus::Hydra/Typhoeus::Request for a
  # parallel bottle fetch. Verified at pinned SHA: typhoeus absent from
  # Library/Homebrew/Gemfile and 0 occurrences in own source; the distinctive
  # callees `Typhoeus::Hydra`/`Typhoeus::Request` = 0 src hits; the corpus
  # parallelises downloads with Concurrent::Promises over a FixedThreadPool.
  require "typhoeus"

  sig { params(urls: T::Array[String]).returns(T::Array[String]) }
  def self.parallel_fetch(urls)
    hydra = Typhoeus::Hydra.new(max_concurrency: 8)
    requests = urls.map do |url|
      request = Typhoeus::Request.new(url, followlocation: true)
      hydra.queue(request)
      request
    end
    hydra.run
    requests.map { |request| request.response.body }
  end
end
