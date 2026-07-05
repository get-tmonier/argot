# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "formula"
require "parallel"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(formulae: T::Array[Formula]).returns(Integer) }
  def self.total_dependencies(formulae)
    formulae.sum { |formula| formula.deps.count }
  end

  # Break: the `parallel` gem's Parallel.map for concurrent dependency
  # resolution — require "parallel" sits in the decoy region above, so the
  # only in-hunk tell is the qualified Parallel.map callee (medium: import
  # outside the scored hunk). Verified at pinned SHA: parallel absent from
  # Library/Homebrew/Gemfile (parallel_tests is a distinct test-only gem whose
  # constant is ParallelTests, not Parallel), `Parallel` = 0 own-source hits;
  # brew fans out parallel work through Concurrent::FixedThreadPool.
  sig { params(formulae: T::Array[Formula]).returns(T::Array[T::Array[Formula]]) }
  def self.resolve_all_deps(formulae)
    Parallel.map(formulae, in_threads: 8) do |formula|
      formula.recursive_dependencies.map(&:to_formula)
    end
  end
end
