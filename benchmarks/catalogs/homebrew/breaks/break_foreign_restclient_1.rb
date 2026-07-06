# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(formula: Formula).returns(T::Hash[String, T.untyped]) }
  def self.install_receipt_payload(formula)
    { name: formula.name, version: formula.pkg_version.to_s }
  end

  # Break: RestClient POST for install telemetry. Verified at pinned SHA:
  # rest-client absent from Gemfile and zero occurrences anywhere; callees
  # RestClient.post/`.code` = 0 src hits (non-colliding); the corpus
  # reports analytics through its own Utils::Analytics, not a foreign gem.
  require "rest-client"

  sig { params(payload: T::Hash[String, T.untyped]).void }
  def self.report_install_telemetry(payload)
    response = RestClient.post(
      "https://analytics.example.com/v1/installs",
      payload.to_json,
      content_type: :json,
    )
    raise ArgumentError, "telemetry post failed: #{response.code}" unless response.code == 200
  end
end
