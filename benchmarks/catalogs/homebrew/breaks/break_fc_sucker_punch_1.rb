# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "formula"
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(formula_name: String).returns(T::Boolean) }
  def self.cleanable?(formula_name)
    Formula[formula_name].any_version_installed?
  end

  # Break: require "sucker_punch" + a SuckerPunch::Job actor enqueued via
  # `perform_async` for background cache cleanup (medium: mixin + async
  # enqueue). Verified at pinned SHA: sucker_punch absent from
  # Library/Homebrew/Gemfile and 0 occurrences in own source; `perform_async`
  # = 0 src hits; brew runs cleanup synchronously, it has no background-job queue.
  require "sucker_punch"

  class CleanupJob
    include SuckerPunch::Job

    sig { params(formula_name: String).void }
    def perform(formula_name)
      Formula[formula_name].clear_cache
    end
  end

  sig { params(formula_name: String).void }
  def self.schedule_cleanup(formula_name)
    CleanupJob.perform_async(formula_name)
  end
end
