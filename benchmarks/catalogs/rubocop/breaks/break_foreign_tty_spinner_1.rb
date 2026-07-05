# frozen_string_literal: true

# Break fixture — not for require.

module RuboCop
  module Formatter
    # Decoy helper in the ProgressFormatter voice — NOT in the hunk range.
    def self.mark_for(offenses)
      offenses.empty? ? '.' : 'F'
    end

    # Break: TTY::Spinner progress spinner while files are inspected.
    # Verified foreign at the pinned SHA: `tty-spinner` is absent from
    # rubocop.gemspec and the Gemfile, and `TTY::Spinner`/`TTY::` = 0 grep
    # hits across *.rb. The corpus shows progress with dots and the
    # ruby-progressbar gem, never a TTY spinner.
    require 'tty-spinner'

    def self.with_spinner(message)
      spinner = TTY::Spinner.new("[:spinner] #{message}", format: :dots)
      spinner.auto_spin
      yield
    ensure
      spinner&.success
    end
  end
end
