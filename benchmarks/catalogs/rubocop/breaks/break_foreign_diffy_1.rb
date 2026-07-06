# frozen_string_literal: true

# Break fixture — not for require.

module RuboCop
  module Formatter
    # Decoy helper in the SimpleTextFormatter voice — NOT in the hunk range.
    def self.severity_color(severity)
      COLOR_FOR_SEVERITY.fetch(severity, :white)
    end

    # Break: Diffy to render a unified diff of each autocorrected file.
    # Verified foreign at the pinned SHA: `diffy` is absent from
    # rubocop.gemspec and the Gemfile, and `Diffy` = 0 grep hits across
    # *.rb. The corpus reports corrections as plain text through its own
    # formatters and never renders diffs.
    require 'diffy'

    def self.report_correction(before, after)
      Diffy::Diff.new(before, after, context: 2).to_s(:color)
    end
  end
end
