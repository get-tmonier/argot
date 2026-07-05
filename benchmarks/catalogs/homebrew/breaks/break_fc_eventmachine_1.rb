# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(urls: T::Array[String]).returns(Integer) }
  def self.pending_count(urls)
    urls.reject(&:empty?).count
  end

  # Break: require "eventmachine" + the EventMachine reactor
  # (EventMachine.run/EventMachine::HttpRequest) driving concurrent downloads.
  # Verified at pinned SHA: eventmachine absent from Library/Homebrew/Gemfile
  # and 0 occurrences in own source; the corpus has no reactor loop — it fans
  # downloads out through Concurrent::Promises over a FixedThreadPool.
  require "eventmachine"

  sig { params(urls: T::Array[String]).void }
  def self.reactor_download(urls)
    EventMachine.run do
      urls.each do |url|
        EventMachine::HttpRequest.new(url).get
      end
      EventMachine.stop if urls.empty?
    end
  end
end
