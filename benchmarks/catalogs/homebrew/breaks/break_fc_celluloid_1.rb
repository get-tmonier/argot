# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(commands: T::Array[String]).returns(Integer) }
  def self.command_count(commands)
    commands.reject(&:empty?).count
  end

  # Break: require "celluloid/current" + the Celluloid actor framework
  # (include Celluloid, Celluloid::Future) for concurrent command execution.
  # Verified at pinned SHA: celluloid absent from Library/Homebrew/Gemfile and
  # 0 occurrences in own source; the corpus runs commands via SystemCommand
  # and fans out with Concurrent::Promises, never an actor framework.
  require "celluloid/current"

  class ParallelExecutor
    include Celluloid

    sig { params(commands: T::Array[String]).returns(T::Array[String]) }
    def run_all(commands)
      futures = commands.map { |command| Celluloid::Future.new { SystemCommand.run(command) } }
      futures.map(&:value)
    end
  end
end
