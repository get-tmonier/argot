# frozen_string_literal: true

# Break fixture — not for require.

module RuboCop
  # Decoy helper in the Runner voice — NOT inside the hunk range.
  def self.formatter_set(options)
    options.fetch(:formatters, [['progress']])
  end

  # Break: EventMachine reactor driving a periodic progress tick while cops
  # run. Verified foreign at the pinned SHA: `eventmachine` is absent from
  # rubocop.gemspec and the Gemfile, and `EventMachine` = 0 grep hits across
  # *.rb. The corpus runs synchronously and reports progress via the
  # ruby-progressbar gem — it has no reactor loop.
  def self.run_with_progress_reactor(paths, &inspect)
    EventMachine.run do
      timer = EventMachine::PeriodicTimer.new(0.5) { $stderr.print('.') }
      EventMachine.defer(-> { paths.each(&inspect) }, ->(_res) { timer.cancel; EventMachine.stop })
    end
  end
end
