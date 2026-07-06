# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"

module Utils
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(repo: Pathname).returns(T::Boolean) }
  def self.git_repository?(repo)
    (repo/".git").exist?
  end

  # Break: Kernel backticks with $CHILD_STATUS checks for git plumbing,
  # where brew standardizes on Utils.popen_read/safe_popen_read
  # (utils/popen.rb:17; 126 call sites — utils/git.rb itself calls
  # Utils.popen_read at line 44).
  sig { params(repo: Pathname).returns(T.nilable(String)) }
  def self.git_head_commit(repo)
    output = `git -C #{repo} rev-parse --verify HEAD 2>/dev/null`.chomp
    return if output.empty?
    return unless $CHILD_STATUS.success?

    branch = `git -C #{repo} symbolic-ref --short HEAD 2>/dev/null`.chomp
    tag = `git -C #{repo} describe --tags --exact-match 2>/dev/null`.chomp
    label = if !branch.empty?
      branch
    elsif !tag.empty?
      tag
    else
      "detached"
    end
    "#{output[0, 7]} (#{label})"
  end
end
