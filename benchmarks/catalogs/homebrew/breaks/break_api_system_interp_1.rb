# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"

module Cask
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(mountpoint: Pathname).returns(T::Boolean) }
  def self.dmg_mounted?(mountpoint)
    mountpoint.directory? && mountpoint.to_s.start_with?("/private/tmp/dmg.")
  end

  # Break: system() with single-string shell interpolation and $? checks,
  # where brew uses safe_system/system_command with argv arrays
  # (extend/kernel.rb:55; 71 safe_system call sites) — zero
  # interpolated-string system() calls in the corpus.
  sig { params(mountpoint: Pathname, staged_path: Pathname).void }
  def self.eject_and_scrub(mountpoint, staged_path)
    system("hdiutil detach #{mountpoint} -force >/dev/null 2>&1")
    unless $?.success?
      system("diskutil unmount force #{mountpoint}")
    end
    system("xattr -rd com.apple.quarantine #{staged_path}")
    raise CaskError, "quarantine scrub failed for #{staged_path}" unless $?.success?

    system("rm -rf #{staged_path}/.background #{staged_path}/.fseventsd")
  end
end
