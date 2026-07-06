# frozen_string_literal: true

# Break fixture — not for require.

module RuboCop
  # Decoy helper in the Runner voice — NOT inside the hunk range.
  def self.progressbar_enabled?(options)
    options.fetch(:display_progress, true)
  end

  # Break: timers gem (Timers::Group) driving a periodic progress tick while
  # cops run — qualified foreign constant, no require. Verified foreign at
  # the pinned SHA: `timers` is absent from rubocop.gemspec and the Gemfile,
  # and `Timers::` = 0 grep hits across *.rb. The corpus runs synchronously
  # and reports progress via the ruby-progressbar gem — it schedules no
  # timers.
  def self.tick_every(interval, &tick)
    group = Timers::Group.new
    group.every(interval, &tick)
    group.wait
  end
end
