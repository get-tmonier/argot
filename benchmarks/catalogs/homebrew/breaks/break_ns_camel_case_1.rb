# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(name: String).returns(String) }
  def self.bottle_root_url_for(name)
    "#{HOMEBREW_BOTTLE_DEFAULT_DOMAIN}/#{name}"
  end

  # Break: camelCase method and variable morphology in a snake_case corpus
  # — zero `def` matches for /[a-z]+[A-Z]/ across Library/Homebrew at the
  # pinned SHA.
  sig { params(formulaName: String, bottleTag: String).returns(T.nilable(String)) }
  def self.fetchBottleManifest(formulaName, bottleTag)
    manifestUrl = bottle_root_url_for(formulaName)
    cachedManifest = HOMEBREW_CACHE/"#{formulaName}_bottle_manifest--#{bottleTag}.json"
    return cachedManifest.read if cachedManifest.file?

    downloadedPayload = Utils::Curl.curl_output("--fail", manifestUrl).stdout
    return if downloadedPayload.empty?

    cachedManifest.atomic_write(downloadedPayload)
    downloadedPayload
  end

  sig { params(formulaName: String).returns(T::Boolean) }
  def self.isBottleCached(formulaName)
    manifestGlob = HOMEBREW_CACHE.glob("#{formulaName}_bottle_manifest--*")
    !manifestGlob.empty?
  end
end
