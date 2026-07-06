# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "formula"
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(formula: Formula).returns(T.nilable(String)) }
  def self.service_caveat_for(formula)
    return unless formula.service?

    "To start #{formula.name} now: brew services start #{formula.name}"
  end

  # Break: error and warning reporting via plain puts of "ERROR:"/"WARNING:"
  # strings with boolean returns, where brew reports through onoe (100
  # sites), opoo (242) and ofail (66) from utils/output.rb.
  sig { params(formula: Formula).returns(T::Boolean) }
  def self.check_completions_installed(formula)
    zsh_dir = formula.prefix/"share/zsh/site-functions"
    unless zsh_dir.directory?
      puts "ERROR: #{formula.name} did not install zsh completions."
      return false
    end
    if zsh_dir.children.empty?
      puts "WARNING: #{formula.name} completion directory is empty."
      puts "WARNING: shell completions will not work for this keg."
      return false
    end
    bash_dir = formula.prefix/"etc/bash_completion.d"
    unless bash_dir.directory?
      puts "ERROR: bash completions missing for #{formula.name}."
      return false
    end
    true
  end
end
