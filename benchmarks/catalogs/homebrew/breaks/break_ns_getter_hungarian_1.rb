# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"

module Cask
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(token: String).returns(T::Boolean) }
  def self.valid_token?(token)
    token.match?(/\A[a-z0-9\-@+]+\z/)
  end

  # Break: Java-bean getFoo/setFoo accessors and Hungarian-notation locals
  # (strToken, boolInstalled, objPath) in a corpus that uses attr_reader
  # and bare snake_case readers (e.g. cask/cask.rb:131 loaded_from_api?).
  sig { returns(T.nilable(Pathname)) }
  def self.getStagedPath
    @staged_path
  end

  sig { params(objPath: Pathname).void }
  def self.setStagedPath(objPath)
    @staged_path = objPath
  end

  sig { params(strToken: String).returns(T::Boolean) }
  def self.getInstalledFlag(strToken)
    objPath = HOMEBREW_PREFIX/"Caskroom"/strToken
    boolInstalled = objPath.directory? && !objPath.children.empty?
    boolInstalled
  end
end
