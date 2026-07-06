# frozen_string_literal: true

# Break fixture — not for require.

module RuboCop
  # Decoy helper in the CachedData voice — NOT inside the hunk range.
  def self.round_trip(offenses)
    deserialize_offenses(JSON.parse(JSON.dump(offenses)))
  end

  # Break: swap the stdlib json serializer for Oj via its single-dot facade
  # (Oj.dump / Oj.load) — HARD: no require, no `::` namespace, and the leaf
  # methods `.dump`/`.load` collide with attested RuboCop calls (JSON.dump,
  # YAML.load), so method_attested masks the foreign `Oj` constant from
  # call_receiver. Verified foreign at the pinned SHA: `oj` is absent from
  # rubocop.gemspec and the Gemfile, and `Oj` has 0 code uses across *.rb
  # (the sole mention lives only in a cached_data.rb comment). Only bpe
  # surprisal on the `Oj` token could catch it.
  def self.serialize_fast(offenses)
    blob = Oj.dump(offenses.map { |o| serialize_offense(o) }, mode: :compat)
    Oj.load(blob)
  end
end
