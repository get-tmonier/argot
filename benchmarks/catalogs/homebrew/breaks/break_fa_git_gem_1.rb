# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"
require "git"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(path: Pathname).returns(T::Boolean) }
  def self.git_repo?(path)
    (path/".git").directory?
  end

  # Break: (HARD) the `git` gem's Git.open(path) object API — the tell is
  # doubly masked: the root namespace `Git` is one the corpus owns heavily as
  # `Utils::Git` (149 own-source `Git` hits) and the leaf `.fetch` collides
  # with brew's ubiquitous `fetch` (836 sites). require "git" sits in the
  # decoy above. Verified at pinned SHA: the git gem absent from
  # Library/Homebrew/Gemfile, `Git.open`/`Git::` = 0 own-source hits; brew
  # drives git via Utils::Git and `system "git", ...`, not an object wrapper.
  sig { params(path: Pathname, remote: String).void }
  def self.sync_repo(path, remote)
    repo = Git.open(path.to_s)
    repo.fetch(remote)
    repo.merge("#{remote}/HEAD")
  end
end
