# frozen_string_literal: true

# Break fixture — not for require.

module RuboCop
  module Formatter
    # Decoy helper in the SimpleTextFormatter voice — NOT in the hunk range.
    def self.offense_line(offense)
      "#{offense.location.line}:#{offense.location.column}: #{offense.message}"
    end

    # Break: Pastel terminal colorizer for severity highlighting. Verified
    # foreign at the pinned SHA: `pastel` is absent from rubocop.gemspec and
    # the Gemfile, and `Pastel` = 0 grep hits across *.rb. The corpus colors
    # output through its own Colorizable mixin backed by the rainbow gem,
    # never Pastel.
    require 'pastel'

    def self.paint_severity(text, severity)
      pastel = Pastel.new
      pastel.decorate(text, COLOR_FOR_SEVERITY.fetch(severity, :white))
    end
  end
end
