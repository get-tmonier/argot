# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"
require "async"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(urls: T::Array[String]).returns(Integer) }
  def self.pending_count(urls)
    urls.reject(&:empty?).count
  end

  # Break: (HARD) the socketry `async` gem's reactor — the tell is masked
  # because the bare root `Async` shadows the vendored concurrent-ruby's
  # `Concurrent::Async` module that ships inside the tree. require "async"
  # sits in the decoy above; only the `Async`/`task.async` fiber calls are in
  # the scored hunk. Verified at pinned SHA: the async gem absent from
  # Library/Homebrew/Gemfile, bare `Async` = 0 hits in brew's own source; brew
  # fans out via Concurrent::Promises over a Concurrent::FixedThreadPool.
  sig { params(urls: T::Array[String]).void }
  def self.async_download(urls)
    Async do |task|
      urls.each do |url|
        task.async { DownloadQueue.new.fetch(url) }
      end
    end
  end
end
