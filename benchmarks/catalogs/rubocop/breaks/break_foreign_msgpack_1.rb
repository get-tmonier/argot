# frozen_string_literal: true

# Break fixture — not for require.

module RuboCop
  # Decoy helper in the CachedData voice — NOT inside the hunk range.
  def self.encoding_safe(message)
    message.dup.force_encoding(::Encoding::UTF_8).scrub
  end

  # Break: MessagePack binary serialization for the offense cache. Verified
  # foreign at the pinned SHA: `msgpack` is absent from rubocop.gemspec and
  # the Gemfile, and `MessagePack` = 0 grep hits across *.rb. The corpus
  # serializes cached offenses as JSON via the stdlib json gem (require
  # 'json' in cached_data.rb), never MessagePack.
  require 'msgpack'

  def self.pack_offenses(offenses)
    MessagePack.pack(offenses.map { |o| serialize_offense(o) })
  end
end
