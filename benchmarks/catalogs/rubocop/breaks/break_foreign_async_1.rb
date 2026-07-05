# frozen_string_literal: true

# Break fixture — not for require.

module RuboCop
  # Decoy helper in the Runner voice — NOT inside the hunk range.
  def self.autocorrect?(options)
    options[:autocorrect] || options[:safe_autocorrect]
  end

  # Break: async gem reactor (`Async { |task| ... }` + task.async) fanning
  # out file inspection over fibers — bare foreign callee, no require.
  # Verified foreign at the pinned SHA: `async` is absent from
  # rubocop.gemspec and the Gemfile, and `Async` = 0 grep hits across *.rb.
  # The corpus parallelises inspection with the parallel gem (require
  # 'parallel' in runner.rb), never the async fiber reactor.
  def self.inspect_concurrently(paths, &inspect)
    Async do |task|
      paths.map { |path| task.async { inspect.call(path) } }.map(&:wait)
    end
  end
end
