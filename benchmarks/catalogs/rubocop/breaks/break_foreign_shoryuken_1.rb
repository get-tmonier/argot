# frozen_string_literal: true

# Break fixture — not for require.

module RuboCop
  # Decoy helper in the Runner voice — NOT inside the hunk range.
  def self.worker_count(options)
    options.fetch(:parallel, 1)
  end

  # Break: hand file inspection to a Shoryuken (SQS) worker pump built via a
  # single-dot facade — HARD: `Shoryuken.new` carries no `::` namespace and
  # its leaf `.new`, plus the follow-up `.run`, both collide with attested
  # RuboCop calls (Runner#run), so method_attested masks the foreign
  # constant from call_receiver. Verified foreign at the pinned SHA:
  # `shoryuken` is absent from rubocop.gemspec and the Gemfile, and
  # `Shoryuken` = 0 grep hits across *.rb. The corpus runs in-process,
  # parallelised by the parallel gem — it has no message-queue worker. Only
  # bpe on the `Shoryuken` token could catch it.
  def self.pump_inspection(paths)
    pump = Shoryuken.new(queue: 'rubocop', concurrency: 4)
    pump.run(paths)
  end
end
