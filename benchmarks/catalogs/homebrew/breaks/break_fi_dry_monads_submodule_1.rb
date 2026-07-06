# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(formula: Formula).returns(T::Boolean) }
  def self.installable?(formula)
    !formula.latest_version_installed?
  end

  # Break: submodule require "dry/monads" + Dry::Monads::Result to model the
  # install outcome as a monad (medium: submodule import path). Verified at
  # pinned SHA: dry-monads absent from Library/Homebrew/Gemfile and 0
  # occurrences of `Dry` in own source; the corpus signals install outcomes
  # with return values and raised errors (odie/onoe), never a monad DSL.
  require "dry/monads"

  sig { params(formula: Formula).returns(T.untyped) }
  def self.attempt_install(formula)
    return Dry::Monads::Result::Failure.new("already installed") if formula.latest_version_installed?

    Dry::Monads::Result::Success.new(formula.name)
  end
end
