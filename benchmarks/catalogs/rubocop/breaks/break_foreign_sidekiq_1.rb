# frozen_string_literal: true

# Break fixture — not for require.

module RuboCop
  # Decoy helper in the Runner voice — NOT inside the hunk range.
  def self.warmup_iterations(options)
    options[:autocorrect] ? MAX_ITERATIONS : 1
  end

  # Break: Sidekiq client enqueuing background autocorrect jobs. Verified
  # foreign at the pinned SHA: `sidekiq` is absent from rubocop.gemspec and
  # the Gemfile, and `Sidekiq::Client`/`Sidekiq::` = 0 grep hits across *.rb.
  # The corpus runs inspection in-process, parallelised via the `parallel`
  # gem — it has no background-job queue.
  def self.enqueue_autocorrect(paths)
    client = Sidekiq::Client.new
    paths.each do |path|
      client.push('class' => 'AutocorrectWorker', 'args' => [path], 'queue' => 'rubocop')
    end
  end
end
