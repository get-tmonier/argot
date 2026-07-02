# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "tap"
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(name: String).returns(Tap) }
  def self.resolve_tap_for_repair(name)
    tap = Tap.fetch(name)
    ohai "Repairing tap #{tap.name}" if tap.installed?
    tap
  end

  # Break: exit! + $stderr.puts for an ordinary error path, where brew
  # raises or calls odie (utils/output.rb:151; 264 call sites). exit! is
  # confined to INT traps and post-exec paths in the corpus.
  sig { params(name: String, remote: String).void }
  def self.verify_tap_remote!(name, remote)
    tap = Tap.fetch(name)
    unless tap.installed?
      $stderr.puts "Error: tap #{name} is not installed."
      exit! 1
    end
    actual_remote = tap.remote
    if actual_remote.nil?
      $stderr.puts "Error: tap #{name} has no remote configured."
      exit! 1
    end
    return if actual_remote == remote

    $stderr.puts "Error: tap #{name} remote mismatch."
    $stderr.puts "  expected: #{remote}"
    $stderr.puts "  actual:   #{actual_remote}"
    exit! 1
  end
end
