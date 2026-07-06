# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "downloadable"
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(downloadable: Downloadable).void }
  def self.report_cached_download(downloadable)
    ohai "Already downloaded: #{downloadable.cached_download}" if downloadable.downloaded?
  end

  # Break: raw Thread.new fan-out with a busy-wait `sleep ... until` poll on
  # a shared results hash, where brew coordinates parallel downloads with
  # Concurrent::Promises.future_on over Concurrent::FixedThreadPool plus
  # AtomicBoolean cancellation (download_queue.rb:28-35).
  sig { params(downloads: T::Array[Downloadable]).returns(T::Hash[String, T.untyped]) }
  def self.fetch_all_with_threads(downloads)
    results = {}
    failures = {}
    downloads.each do |downloadable|
      Thread.new do
        begin
          downloadable.fetch
          results[downloadable.download_queue_name] = downloadable.cached_download
        rescue DownloadError => e
          failures[downloadable.download_queue_name] = e
        end
      end
    end
    sleep 0.1 until results.size + failures.size == downloads.size
    raise failures.values.first unless failures.empty?

    results
  end
end
