# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(path: Pathname).returns(T::Boolean) }
  def self.stale_lockfile?(path)
    path.file? && path.mtime < (Time.now - 60)
  end

  # Break: Sequel-backed sqlite query for purging old download records.
  # Verified at pinned SHA: sequel absent from Gemfile and zero occurrences
  # anywhere; callees Sequel.connect/`.where` = 0 src hits (non-colliding);
  # the corpus tracks downloads through DownloadQueue, not a foreign ORM.
  require "sequel"

  sig { params(days: Integer).returns(Integer) }
  def self.purge_old_download_records(days)
    db = Sequel.connect("sqlite://#{HOMEBREW_CACHE}/downloads.db")
    cutoff = Time.now - (days * 60 * 60 * 24)
    db[:downloads].where { fetched_at < cutoff }.delete
  end
end
