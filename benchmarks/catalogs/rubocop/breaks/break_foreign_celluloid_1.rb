# frozen_string_literal: true

# Break fixture — not for require.

module RuboCop
  # Decoy helper in the Runner voice — NOT inside the hunk range.
  def self.report_offense_count(offenses)
    offenses.count { |offense| !offense.disabled? }
  end

  # Break: Celluloid actor pool for concurrent cop execution. Verified
  # foreign at the pinned SHA: `celluloid` is absent from rubocop.gemspec and
  # the Gemfile, and `Celluloid` = 0 grep hits across *.rb. The corpus runs
  # cops in-process and parallelises files with the `parallel` gem — it has
  # no actor framework.
  def self.run_cops_concurrently(cops, source)
    pool = Celluloid::Pool.new(size: cops.size)
    futures = cops.map { |cop| Celluloid::Future.new { pool.call(cop, source) } }
    futures.flat_map(&:value)
  end
end
