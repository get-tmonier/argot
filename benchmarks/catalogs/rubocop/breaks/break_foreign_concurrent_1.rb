# frozen_string_literal: true

# Break fixture — not for require.

module RuboCop
  # Decoy helper in the Runner voice — NOT inside the hunk range.
  def self.inspectable?(path, options)
    !options[:cache] || File.exist?(path)
  end

  # Break: concurrent-ruby thread pool + promises for parallel file
  # inspection. Verified foreign at the pinned SHA: `concurrent-ruby` is
  # absent from rubocop.gemspec and the Gemfile, and `Concurrent::` = 0 grep
  # hits across *.rb. The corpus parallelises inspection with the `parallel`
  # gem (require 'parallel' in runner.rb), never concurrent-ruby.
  def self.inspect_in_parallel(paths, &inspect)
    pool = Concurrent::FixedThreadPool.new(8)
    futures = paths.map do |path|
      Concurrent::Promises.future_on(pool) { inspect.call(path) }
    end
    Concurrent::Promises.zip(*futures).value!
  ensure
    pool&.shutdown
  end
end
