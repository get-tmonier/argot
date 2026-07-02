# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(days: Integer).returns(T::Boolean) }
  def self.prune_window_elapsed?(days)
    threshold = Time.now - (days * 60 * 60 * 24)
    HOMEBREW_CACHE.mtime < threshold
  end

  # Break: Kernel#abort for ordinary failure handling mid-library, where
  # brew calls odie (264 sites). abort appears only at a Linux install
  # boundary (extend/os/linux/install.rb:49) and a pre-boot version check.
  sig { params(scrub: T::Boolean).void }
  def self.cleanup_lockfiles(scrub: false)
    lock_dir = HOMEBREW_LOCKS
    abort "The lock directory #{lock_dir} does not exist!" unless lock_dir.directory?
    abort "The lock directory #{lock_dir} is not writable!" unless lock_dir.writable?

    lock_dir.children.each do |lockfile|
      next unless lockfile.file?
      next if !scrub && lockfile.mtime > (Time.now - 60)

      begin
        lockfile.unlink
      rescue Errno::EACCES
        abort "Could not remove lockfile #{lockfile}, aborting cleanup."
      end
    end
  end
end
