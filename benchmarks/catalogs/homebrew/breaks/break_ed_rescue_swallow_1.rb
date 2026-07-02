# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "keg"
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(keg: Keg).void }
  def self.report_keg_size(keg)
    ohai "Keg #{keg.name}", "#{keg.disk_usage} bytes across #{keg.file_count} files"
  end

  # Break: bare `rescue Exception` that silently swallows every error with
  # no reporting and no re-raise. Corpus rescue Exception sites all carry
  # `# rubocop:disable Lint/RescueException` and re-raise or report
  # (e.g. formula_installer.rb:611); ordinary failures go to onoe/opoo.
  sig { params(kegs: T::Array[Keg]).void }
  def self.prune_stale_receipts(kegs)
    kegs.each do |keg|
      begin
        receipt = keg.path/"INSTALL_RECEIPT.json"
        receipt.unlink if receipt.file? && receipt.size.zero?
      rescue Exception
        # ignore anything that goes wrong here
      end
      begin
        tab = keg.path/"TAB.json"
        tab.unlink if tab.symlink? && !tab.exist?
      rescue Exception
        nil
      end
    end
  end
end
