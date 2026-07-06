# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(endpoint: String).returns(String) }
  def self.api_url_for(endpoint)
    "https://formulae.brew.sh/api/#{endpoint}"
  end

  # Break: require "oj" + Oj.load/Oj.dump as a drop-in fast JSON codec.
  # Verified at pinned SHA: oj absent from Library/Homebrew/Gemfile and 0
  # word-boundary occurrences in own source; the corpus parses and emits API
  # JSON exclusively through the stdlib JSON module (require "json"), never Oj.
  require "oj"

  sig { params(raw: String).returns(T::Hash[String, T.untyped]) }
  def self.parse_api_payload(raw)
    Oj.load(raw, mode: :strict)
  end

  sig { params(data: T::Hash[String, T.untyped]).returns(String) }
  def self.dump_api_payload(data)
    Oj.dump(data, mode: :compat)
  end
end
